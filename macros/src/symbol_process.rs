use std::cmp::Ordering;

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use smallvec::{SmallVec, smallvec};
use syn::{
  Expr, ExprField, ExprMethodCall, Macro, Member,
  fold::Fold,
  parse::{Parse, ParseStream},
  parse_quote, parse_quote_spanned,
  spanned::Spanned,
  token::Dollar,
};

use crate::{
  fn_widget_macro,
  rdl_macro::RdlMacro,
  variable_names::{BUILTIN_INFOS, BuiltinMember, BuiltinMemberType, ribir_suffix_variable},
};

pub const KW_DOLLAR_STR: &str = "_dollar_ಠ_ಠ";
pub const KW_CTX: &str = "ctx";
pub const KW_RDL: &str = "rdl";
pub const KW_PIPE: &str = "pipe";
pub const KW_DISTINCT_PIPE: &str = "distinct_pipe";
pub const KW_WATCH: &str = "watch";
pub const KW_FN_WIDGET: &str = "fn_widget";

pub use tokens_pre_process::*;

pub mod kw {
  syn::custom_keyword!(_dollar_ಠ_ಠ);
  syn::custom_keyword!(rdl);
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct BuiltinInfo {
  pub(crate) host: Ident,
  pub(crate) get_builtin: Ident,
  pub(crate) run_before_clone: SmallVec<[Ident; 1]>,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, PartialOrd, Ord)]
pub enum DollarUsedInfo {
  Reader,
  Watcher,
  Writer,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct DollarRef {
  pub name: Ident,
  pub builtin: Option<BuiltinInfo>,
  pub used: DollarUsedInfo,
}
#[derive(Debug)]
pub struct DollarRefsCtx {
  scopes: SmallVec<[DollarRefsScope; 1]>,
  variable_stacks: Vec<Vec<Ident>>,
}

#[derive(Debug, Default)]
pub struct DollarRefsScope {
  refs: SmallVec<[DollarRef; 1]>,
  pub(crate) used_ctx: bool,
  /// The index of the head of this scope in the variable stack.
  variable_stack_head: usize,
  /// This scope will exclusively capture the specified variable and will not be
  /// treated as a real scope. If set to `None`, it will capture all dollar
  /// references used.
  only_capture: Option<Ident>,
}

pub struct StackGuard<'a>(&'a mut DollarRefsCtx);

mod tokens_pre_process {
  use proc_macro2::*;

  use super::KW_DOLLAR_STR;
  use crate::{error::*, symbol_process::KW_RDL};

  /// Convert `@` and `$` symbol to a `rdl!` or `_dollar_ಠ_ಠ!` macro, make it
  /// conform to Rust syntax
  pub fn symbol_to_macro(input: TokenStream) -> Result<TokenStream> {
    let mut iter = input.into_iter();
    let mut tokens = vec![];

    loop {
      match iter.next() {
        Some(TokenTree::Punct(at))
          // maybe rust identify bind syntax, `identify @`
          if at.as_char() == '@' && !matches!(tokens.last(), Some(TokenTree::Ident(_))) =>
        {
          tokens.push(TokenTree::Ident(Ident::new(KW_RDL, at.span())));
          tokens.push(not_token(at.span()));
          let mut rdl_group = smallvec::SmallVec::<[TokenTree; 3]>::default();
           match iter.next() {
            // declare a variable widget as parent,  `@ $var { ... }`
            Some(TokenTree::Punct(dollar)) if dollar.as_char() == '$' => {
              if let Some(TokenTree::Ident(var)) = iter.next() {
                rdl_group.push(TokenTree::Punct(dollar));
                rdl_group.push(TokenTree::Ident(var));
                if let Some(g)  = iter.next()  {
                 rdl_group.push(g);
                };
              } else {
                return Err(Error::IdentNotFollowDollar(dollar.span()));
              }
            }
            // declare a expression widget  `@ { ... }`
            Some(TokenTree::Group(g)) => rdl_group.push(TokenTree::Group(g)) ,
            // declare a new widget: `@ SizedBox { ... }`
            mut n => {
              while let Some(t) = n.take() {
                let is_group = matches!(t, TokenTree::Group(_));
                rdl_group.push(t);
                if is_group {
                  break
                }
                n = iter.next();
              };
            },
          };
          if let Some(TokenTree::Group(_)) = rdl_group.last()  {
            let rdl_group = TokenStream::from_iter(rdl_group);
            tokens.push(TokenTree::Group(Group::new(Delimiter::Brace, rdl_group)));
          } else {
            let follow = rdl_group.last().map(|n| n.span());
            return Err(Error::RdlAtSyntax{at: at.span(), follow});
          }
        }
        Some(TokenTree::Punct(p)) if p.as_char() == '$' => {
          match iter.next() {
            Some(TokenTree::Ident(name)) => {
              tokens.push(TokenTree::Ident(Ident::new(KW_DOLLAR_STR, p.span())));
              tokens.push(not_token(p.span()));
              let span = name.span();
              let mut g = Group::new(
                Delimiter::Parenthesis,
                [TokenTree::Punct(p), TokenTree::Ident(name)].into_iter().collect()
              );
              g.set_span(span);
              tokens.push(TokenTree::Group(g));
            }
            Some(t) =>     return Err(Error::IdentNotFollowDollar(t.span())),
            None =>   return Err(Error::IdentNotFollowDollar(p.span())),
          };
        }
        Some(TokenTree::Group(mut g)) => {
          // not process symbol in macro, because it's maybe as part of the macro syntax.
          if !in_macro(&tokens) {
            let mut n = Group::new(g.delimiter(), symbol_to_macro(g.stream())?);
            n.set_span(g.span());
            g = n;
          }

          tokens.push(TokenTree::Group(g));
        }
        Some(t) => tokens.push(t),
        None => break,
      };
    }
    Ok(tokens.into_iter().collect())
  }

  fn in_macro(tokens: &[TokenTree]) -> bool {
    let [.., TokenTree::Ident(_), TokenTree::Punct(p)] = tokens else {
      return false;
    };
    p.as_char() == '!'
  }

  fn not_token(span: Span) -> TokenTree {
    let mut t = Punct::new('!', Spacing::Alone);
    t.set_span(span);
    TokenTree::Punct(t)
  }
}

impl Fold for DollarRefsCtx {
  fn fold_block(&mut self, i: syn::Block) -> syn::Block {
    let mut this = self.push_code_stack();
    syn::fold::fold_block(&mut *this, i)
  }

  fn fold_expr_closure(&mut self, i: syn::ExprClosure) -> syn::ExprClosure {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_closure(&mut *this, i)
  }

  fn fold_item_const(&mut self, i: syn::ItemConst) -> syn::ItemConst {
    self.new_local_var(&i.ident);
    syn::fold::fold_item_const(self, i)
  }

  fn fold_local(&mut self, mut i: syn::Local) -> syn::Local {
    //  we fold right expression first, then fold pattern, because the `=` is a
    // right operator.
    i.init = i.init.map(|init| self.fold_local_init(init));
    i.pat = self.fold_pat(i.pat);
    i
  }

  fn fold_expr_block(&mut self, i: syn::ExprBlock) -> syn::ExprBlock {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_block(&mut *this, i)
  }

  fn fold_expr_for_loop(&mut self, i: syn::ExprForLoop) -> syn::ExprForLoop {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_for_loop(&mut *this, i)
  }

  fn fold_expr_loop(&mut self, i: syn::ExprLoop) -> syn::ExprLoop {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_loop(&mut *this, i)
  }

  fn fold_expr_if(&mut self, i: syn::ExprIf) -> syn::ExprIf {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_if(&mut *this, i)
  }

  fn fold_arm(&mut self, i: syn::Arm) -> syn::Arm {
    let mut this = self.push_code_stack();
    syn::fold::fold_arm(&mut *this, i)
  }

  fn fold_expr_unsafe(&mut self, i: syn::ExprUnsafe) -> syn::ExprUnsafe {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_unsafe(&mut *this, i)
  }

  fn fold_expr_while(&mut self, i: syn::ExprWhile) -> syn::ExprWhile {
    let mut this = self.push_code_stack();
    syn::fold::fold_expr_while(&mut *this, i)
  }

  fn fold_pat_ident(&mut self, i: syn::PatIdent) -> syn::PatIdent {
    self.new_local_var(&i.ident);
    syn::fold::fold_pat_ident(self, i)
  }

  fn fold_expr_field(&mut self, mut i: ExprField) -> ExprField {
    let ExprField { base, member, .. } = &mut i;

    if let Member::Named(member) = member {
      let dollar = BUILTIN_INFOS
        .get(member.to_string().as_str())
        .filter(|info| info.mem_ty == BuiltinMemberType::Field)
        .and_then(|info| self.replace_builtin_ident(&mut *base, info));
      if dollar.is_some() {
        return i;
      }
    }

    syn::fold::fold_expr_field(self, i)
  }

  fn fold_expr_method_call(&mut self, mut i: ExprMethodCall) -> ExprMethodCall {
    // fold builtin method on state
    if BUILTIN_INFOS
      .get(&i.method.to_string())
      .filter(|info| info.mem_ty == BuiltinMemberType::Method)
      .and_then(|info| self.replace_builtin_ident(&mut i.receiver, info))
      .is_some()
    {
      return i;
    }

    // fold if write on state.
    if let Expr::Macro(m) = &mut *i.receiver {
      if is_state_write_method(&i.method) {
        if let Some(d) = parse_dollar_macro(&m.mac) {
          let name = d.name;
          m.mac.tokens = expand_write_method(name.to_token_stream());
          mark_macro_expanded(&mut m.mac);
          let dollar_ref = DollarRef { name, builtin: None, used: DollarUsedInfo::Writer };
          self.add_dollar_ref(dollar_ref);
          return i;
        }
      }
    }

    syn::fold::fold_expr_method_call(self, i)
  }

  fn fold_macro(&mut self, mut mac: Macro) -> Macro {
    if let Some(DollarMacro { name, .. }) = parse_dollar_macro(&mac) {
      mac.tokens = expand_read(name.to_token_stream());
      mark_macro_expanded(&mut mac);
      let dollar_ref = DollarRef { name, builtin: None, used: DollarUsedInfo::Reader };
      self.add_dollar_ref(dollar_ref)
    } else if mac.path.is_ident(KW_WATCH) {
      mac.tokens = crate::watch_macro::gen_code(mac.tokens, self);
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_PIPE) {
      mac.tokens = crate::pipe_macro::gen_code(mac.tokens, self);
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_DISTINCT_PIPE) {
      mac.tokens = crate::distinct_pipe_macro::gen_code(mac.tokens, self).into();
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_RDL) {
      self.mark_used_ctx();
      mac.tokens = RdlMacro::gen_code(mac.tokens, self);
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_FN_WIDGET) {
      mac.tokens = fn_widget_macro::gen_code(mac.tokens, self);
      mark_macro_expanded(&mut mac);
    } else if mac.path.is_ident(KW_CTX) {
      self.mark_used_ctx();
    } else {
      mac = syn::fold::fold_macro(self, mac);
    }
    mac
  }

  fn fold_expr(&mut self, i: Expr) -> Expr {
    match i {
      Expr::Closure(c) if c.capture.is_some() => {
        self.new_dollar_scope(None);
        let mut c = self.fold_expr_closure(c);
        let dollar_scope = self.pop_dollar_scope(false);

        if dollar_scope.used_ctx() || !dollar_scope.is_empty() {
          if dollar_scope.used_ctx() {
            let body = &mut *c.body;
            let body_with_ctx = quote_spanned! { body.span() =>
              _ctx_handle.with_ctx(|ctx!()| #body).expect("ctx is not available")
            };

            if matches!(c.output, syn::ReturnType::Default) {
              *body = parse_quote! { #body_with_ctx };
            } else {
              *body = parse_quote_spanned! { body.span() => { #body_with_ctx }};
            }
          }

          let handle = dollar_scope
            .used_ctx()
            .then(|| quote_spanned! { c.span() => let _ctx_handle = ctx!().handle(); });

          Expr::Verbatim(quote_spanned!(c.span() => {
            #dollar_scope
            #handle
            #c
          }))
        } else {
          Expr::Closure(self.fold_expr_closure(c))
        }
      }
      _ => syn::fold::fold_expr(self, i),
    }
  }
}

fn mark_macro_expanded(mac: &mut Macro) {
  mac.path = parse_quote_spanned! { mac.path.span() => ribir_expanded_ಠ_ಠ };
}

impl ToTokens for DollarRefsScope {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.refs.iter().for_each(|d| d.to_tokens(tokens))
  }
}

impl ToTokens for DollarRef {
  fn to_tokens(&self, tokens: &mut TokenStream) { self.capture_state(&self.name, tokens); }
}

impl DollarRef {
  pub fn capture_state(&self, var_name: &Ident, tokens: &mut TokenStream) {
    let Self { name, builtin, used } = self;

    if let Some(BuiltinInfo { host, get_builtin: member, run_before_clone }) = builtin {
      quote_spanned! { name.span() =>
        let #var_name = #host #(.#run_before_clone())* .#member()
      }
    } else {
      quote_spanned! { name.span() => let #var_name = #name }
    }
    .to_tokens(tokens);
    let span = name.span();
    match used {
      DollarUsedInfo::Reader => quote_spanned! { span => .clone_reader() },
      DollarUsedInfo::Watcher => quote_spanned! { span => .clone_watcher() },
      DollarUsedInfo::Writer => quote_spanned! { span => .clone_writer() },
    }
    .to_tokens(tokens);
    syn::token::Semi(name.span()).to_tokens(tokens);
  }
}

impl DollarRefsCtx {
  #[inline]
  pub fn top_level() -> Self { Self::default() }

  /// Begin a new scope to track dollar reference information.
  #[inline]
  pub fn new_dollar_scope(&mut self, only_capture: Option<Ident>) {
    let variable_stack_head = if only_capture.is_some() {
      self.current_dollar_scope().variable_stack_head
    } else {
      self.variable_stacks.push(vec![]);
      self.variable_stacks.len() - 1
    };

    self.scopes.push(DollarRefsScope {
      only_capture,
      refs: <_>::default(),
      used_ctx: false,
      variable_stack_head,
    });
  }

  /// Pop the last dollar scope, and removes duplicate elements in it and make
  /// the builtin widget first. Keep the builtin reference before the host
  /// because if a obj both reference builtin widget and its host, the host
  /// reference may shadow the original.
  ///
  /// For example, this generate code not work:
  ///
  /// ```ignore
  /// let a = a.clone_reader();
  /// // the `a` is shadowed by the before `a` variable.
  /// let a_margin = a.get_margin_widget(ctx!());
  /// ```
  ///
  /// must generate `a_margin` first:
  ///
  /// ```ignore
  /// let a_margin = a.get_margin_widget(ctx!());
  /// let a = a.clone_reader();
  /// ```
  ///
  /// - **watch_scope**: A watch scope that should designate all readers as
  ///   watchers.
  pub fn pop_dollar_scope(&mut self, watch_scope: bool) -> DollarRefsScope {
    let mut scope = self.scopes.pop().unwrap();
    if scope.only_capture.is_none() {
      self.variable_stacks.pop();
    }

    // To maintain the order, ensure that the builtin widget precedes its host.
    // Otherwise, the host might only clone a reader that cannot create a
    // builtin widget.
    scope
      .refs
      .sort_by(|a, b| a.builtin.is_none().cmp(&b.builtin.is_none()));

    if !self.scopes.is_empty() {
      self.current_dollar_scope_mut().used_ctx |= scope.used_ctx();

      for r in scope.refs.iter_mut() {
        if self.is_capture_var(r.host()) {
          let mut c_r = r.clone();
          if watch_scope && c_r.used == DollarUsedInfo::Reader {
            c_r.used = DollarUsedInfo::Watcher;
          }
          self.add_dollar_ref(c_r);

          // If a built-in widget is captured by the parent scope and treated as a regular
          // variable, the child scope does not need to capture it from the host variable.
          r.builtin.take();
        }
      }
    }

    if let Some(c) = scope.only_capture.as_ref() {
      scope.refs.drain_filter(|r| r.host() != c);
    }
    scope
  }

  pub fn push_code_stack(&mut self) -> StackGuard<'_> {
    self.variable_stacks.push(vec![]);
    StackGuard(self)
  }

  pub fn builtin_host_tokens(&self, dollar_ref: &DollarRef) -> TokenStream {
    let DollarRef { name, builtin, .. } = dollar_ref;
    let BuiltinInfo { host, get_builtin: member, run_before_clone } = builtin.as_ref().unwrap();

    // if used in embedded closure, we directly use the builtin variable, the
    // variable that capture by the closure is already a separate builtin variable.
    if !self.is_local_var(host) {
      name.to_token_stream()
    } else {
      quote_spanned! { host.span() => #host #(.#run_before_clone())*.#member() }
    }
  }

  fn mark_used_ctx(&mut self) { self.current_dollar_scope_mut().used_ctx = true; }

  fn replace_builtin_ident(
    &mut self, caller: &mut Expr, info: &BuiltinMember,
  ) -> Option<&DollarRef> {
    let mut used = DollarUsedInfo::Reader;
    let e = if let Expr::MethodCall(m) = caller {
      if is_state_write_method(&m.method) {
        used = DollarUsedInfo::Writer;
        &mut *m.receiver
      } else {
        caller
      }
    } else {
      caller
    };

    let Expr::Macro(m) = e else { return None };
    let DollarMacro { name: host, .. } = parse_dollar_macro(&m.mac)?;

    // When a builtin widget captured by a `move |_| {...}` closure, we need split
    // the builtin widget from the `FatObj` so we only capture the builtin part that
    // we used.
    let name = ribir_suffix_variable(&host, info.var_name);
    let get_builtin = Ident::new(&format!("get_{}_widget", info.var_name), host.span());
    let mut run_before_clone = SmallVec::default();
    if let Some(method) = info.run_before_clone {
      run_before_clone.push(Ident::new(method, host.span()));
    }
    Ident::new(&format!("get_{}_widget", info.var_name), host.span());
    let builtin = Some(BuiltinInfo { host, get_builtin, run_before_clone });
    let dollar_ref = DollarRef { name, builtin, used };

    let state = self.builtin_host_tokens(&dollar_ref);
    m.mac.tokens = if dollar_ref.used == DollarUsedInfo::Writer {
      expand_write_method(state)
    } else {
      expand_read(state)
    };
    mark_macro_expanded(&mut m.mac);

    self.add_dollar_ref(dollar_ref);
    self.current_dollar_scope().last()
  }

  fn new_local_var(&mut self, name: &Ident) {
    self
      .variable_stacks
      .last_mut()
      .unwrap()
      .push(name.clone())
  }

  fn add_dollar_ref(&mut self, dollar_ref: DollarRef) {
    // local variable is not a outside reference.
    if self.is_capture_var(dollar_ref.host()) {
      let scope = self.current_dollar_scope_mut();
      let r = scope
        .refs
        .iter_mut()
        .find(|v| v.name == dollar_ref.name);
      if let Some(r) = r {
        if r.used.cmp(&dollar_ref.used) == Ordering::Less {
          r.used = dollar_ref.used
        }
        if let (Some(this), Some(other)) = (r.builtin.as_mut(), dollar_ref.builtin) {
          for e in other.run_before_clone {
            if this.run_before_clone.iter().any(|e2| &e != e2) {
              this.run_before_clone.push(e);
            }
          }
        }
      } else {
        scope.refs.push(dollar_ref);
      }
    }
  }

  pub fn current_dollar_scope(&self) -> &DollarRefsScope {
    self.scopes.last().expect("no dollar refs scope")
  }

  pub fn current_dollar_scope_mut(&mut self) -> &mut DollarRefsScope {
    self
      .scopes
      .last_mut()
      .expect("no dollar refs scope")
  }

  fn is_only_capture(&self, name: &Ident) -> bool {
    self.current_dollar_scope().only_capture.as_ref() == Some(name)
  }

  fn is_capture_var(&self, name: &Ident) -> bool {
    self.is_only_capture(name) || !self.is_local_var(name)
  }

  fn is_local_var(&self, name: &Ident) -> bool {
    let head = self.current_dollar_scope().variable_stack_head;
    self.variable_stacks[head..]
      .iter()
      .any(|stack| stack.contains(name))
  }
}

impl DollarRefsScope {
  pub fn used_ctx(&self) -> bool { self.used_ctx }

  pub fn upstream_tokens(&self) -> TokenStream {
    match self.len() {
      0 => quote! {},
      1 => {
        let upstream = self.refs[0].upstream_tokens();
        quote! { observable::of(ModifyScope::DATA).merge(#upstream) }
      }
      _ => {
        let upstream = self.iter().map(DollarRef::upstream_tokens);
        quote_spanned! { self.refs[0].name.span() =>
          observable::of(ModifyScope::DATA)
            .merge(observable::from_iter([#(#upstream),*]).merge_all(usize::MAX))
        }
      }
    }
  }
}

impl DollarRef {
  pub fn host(&self) -> &Ident {
    self
      .builtin
      .as_ref()
      .map_or_else(|| &self.name, |b| &b.host)
  }

  pub fn upstream_tokens(&self) -> TokenStream {
    let DollarRef { name, builtin, .. } = self;
    if let Some(BuiltinInfo { host, get_builtin: member, .. }) = builtin {
      quote_spanned! { name.span() => #host.#member().modifies() }
    } else {
      quote_spanned! { name.span() => #name.modifies() }
    }
  }
}

fn parse_dollar_macro(mac: &Macro) -> Option<DollarMacro> {
  if mac.path.is_ident(KW_DOLLAR_STR) {
    Some(mac.parse_body::<DollarMacro>().unwrap())
  } else {
    None
  }
}

impl std::ops::Deref for DollarRefsScope {
  type Target = [DollarRef];
  fn deref(&self) -> &Self::Target { &self.refs }
}

struct DollarMacro {
  _dollar: Dollar,
  name: Ident,
}

impl Parse for DollarMacro {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let _dollar = input.parse()?;
    let name = if input.peek(syn::token::SelfValue) {
      let name = input.parse::<syn::token::SelfValue>()?;
      Ident::new("self", name.span())
    } else {
      input.parse::<Ident>()?
    };

    Ok(Self { _dollar, name })
  }
}

impl<'a> std::ops::Deref for StackGuard<'a> {
  type Target = DollarRefsCtx;
  fn deref(&self) -> &Self::Target { self.0 }
}

impl<'a> std::ops::DerefMut for StackGuard<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

impl<'a> Drop for StackGuard<'a> {
  fn drop(&mut self) { self.0.variable_stacks.pop(); }
}

impl Default for DollarRefsCtx {
  fn default() -> Self { Self { scopes: smallvec![], variable_stacks: vec![vec![]] } }
}

fn is_state_write_method(m: &Ident) -> bool { m == "write" || m == "silent" || m == "shallow" }

fn expand_write_method(host: TokenStream) -> TokenStream { host }

fn expand_read(name: TokenStream) -> TokenStream { quote_spanned!(name.span() => #name.read()) }
