use crate::attr_fields::{add_trait_bounds_if, AttrFields};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{parse_quote, Data, DeriveInput, Generics, Ident};
pub const PROXY_PATH: &str = "proxy";

pub struct ProxyDeriveInfo<'a> {
  pub derive_trait: &'static str,
  pub attr_fields: AttrFields<'a>,
  pub ident: &'a Ident,
  pub generics: &'a Generics,
}

impl<'a> ProxyDeriveInfo<'a> {
  pub fn new(input: &'a syn::DeriveInput, derive_trait: &'static str) -> Result<Self, TokenStream> {
    let DeriveInput { ident, data, generics, .. } = input;

    match data {
      Data::Struct(stt) => {
        let attr_fields = AttrFields::new(&stt, generics, PROXY_PATH);
        Ok(Self {
          derive_trait,
          attr_fields,
          ident,
          generics,
        })
      }
      _ => Err(quote_spanned! {
        ident.span() => compile_error!("`{}` can not derived by this type.", derive_trait);
      }),
    }
  }

  pub fn proxy_path(&self) -> TokenStream {
    let (f, idx) = &self.attr_fields.attr_fields()[0];
    let path = f.ident.as_ref().map_or_else(
      || {
        let index = syn::Index::from(*idx);
        quote! {#index}
      },
      |f| quote! {#f},
    );
    path
  }

  pub fn too_many_proxy_specified_error(self) -> Result<Self, TokenStream> {
    if self.attr_fields.attr_fields().len() > 1 {
      Err(quote_spanned! {
       self.ident.span() => compile_error!("`{}` can not derived by this type.", self.derive_trait);
      })
    } else {
      Ok(self)
    }
  }

  pub fn none_proxy_specified_error(self) -> Result<Self, TokenStream> {
    if self.attr_fields.attr_fields().len() == 0 {
      Err(quote_spanned! {
       self. ident.span() => compile_error!("Must specify a `#[proxy] attr to one field.");
      })
    } else {
      Ok(self)
    }
  }
}

pub fn widget_derive(input: &syn::DeriveInput) -> TokenStream {
  let info =
    ProxyDeriveInfo::new(input, "Widget").and_then(|stt| stt.too_many_proxy_specified_error());

  match info {
    Ok(info) => {
      let name = info.ident;
      let attr_fields = &info.attr_fields;

      let mut generics = info.generics.clone();

      let (attrs_ref_impl, attrs_mut_impl) = if attr_fields.attr_fields().len() == 1 {
        generics = add_trait_bounds_if(generics, parse_quote!(Widget), |param| {
          attr_fields.is_attr_generic(param)
        });
        let path = info.proxy_path();
        (
          quote! { self.#path.attrs_ref() },
          quote! {self.#path.attrs_mut()},
        )
      } else {
        if let Some(ref mut w) = generics.where_clause {
          w.predicates.push(parse_quote!(Self:'static));
        } else {
          generics.where_clause = parse_quote!(where Self: 'static)
        }

        (quote! {None}, quote! {None})
      };

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      quote! {
          impl #impl_generics Widget for #name #ty_generics #where_clause {
            #[inline]
            fn attrs_ref(&self) -> Option<AttrsRef> { #attrs_ref_impl }

            #[inline]
            fn attrs_mut(&mut self) -> Option<AttrsMut> { #attrs_mut_impl }
          }

          impl #impl_generics AttachAttr for #name #ty_generics #where_clause {
            type W = Self;

            fn take_attr<A: Any>(self) -> (Option<A>, Option<Attrs>, Self::W) {
              (None, None, self)
            }
          }
      }
    }
    Err(err) => err,
  }
}