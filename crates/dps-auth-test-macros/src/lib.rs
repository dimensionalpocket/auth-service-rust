use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, ItemFn, LitBool, LitInt, Meta, Token};

#[derive(Default)]
struct DbTestArgs {
  crate_path: Option<syn::Path>,
  configure_sqlite: Option<bool>,
  pool_size: Option<u32>,
}

impl Parse for DbTestArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let metas = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
    let mut out = DbTestArgs::default();

    for meta in metas {
      match meta {
        Meta::NameValue(nv) if nv.path.is_ident("crate_path") => {
          let path = match nv.value {
            syn::Expr::Path(expr_path) => expr_path.path,
            other => {
              return Err(syn::Error::new_spanned(
                other,
                "crate_path must be a path (e.g. crate_path = dps_auth_api)",
              ))
            }
          };

          out.crate_path = Some(path);
        }
        Meta::NameValue(nv) if nv.path.is_ident("configure_sqlite") => {
          let lit = match nv.value {
            syn::Expr::Lit(expr_lit) => expr_lit.lit,
            other => {
              return Err(syn::Error::new_spanned(
                other,
                "configure_sqlite must be a boolean literal",
              ))
            }
          };

          let LitBool { value, .. } = syn::parse2::<LitBool>(quote::quote!(#lit))?;
          out.configure_sqlite = Some(value);
        }
        Meta::NameValue(nv) if nv.path.is_ident("pool_size") => {
          let lit = match nv.value {
            syn::Expr::Lit(expr_lit) => expr_lit.lit,
            other => {
              return Err(syn::Error::new_spanned(
                other,
                "pool_size must be an integer literal",
              ))
            }
          };

          let LitInt { .. } = syn::parse2::<LitInt>(quote::quote!(#lit))?;
          let pool_size: u32 = syn::parse2::<LitInt>(quote::quote!(#lit))?.base10_parse()?;
          out.pool_size = Some(pool_size);
        }
        other => {
          return Err(syn::Error::new_spanned(
            other,
            "expected: crate_path = <path>, configure_sqlite = <bool>, pool_size = <int>",
          ))
        }
      }
    }

    Ok(out)
  }
}

#[proc_macro_attribute]
pub fn dps_auth_db_test(attr: TokenStream, item: TokenStream) -> TokenStream {
  let args = parse_macro_input!(attr as DbTestArgs);
  let input_fn = parse_macro_input!(item as ItemFn);

  let sig = &input_fn.sig;
  let vis = &input_fn.vis;
  let attrs = &input_fn.attrs;
  let block = &input_fn.block;
  let name = &sig.ident;

  let configure_sqlite = args.configure_sqlite.unwrap_or(true);
  let pool_size = args.pool_size.unwrap_or(1);
  // Default to `crate` so unit tests can work without extra args.
  // For integration tests (tests/*.rs), either pass `crate_path = dps_auth_api`
  // or alias `dps_auth_api::test_utils` as `test_utils` in the test crate.
  let crate_path: syn::Path = args.crate_path.unwrap_or_else(|| syn::parse_quote!(crate));

  let expanded = quote! {
    #(#attrs)*
    #[tokio::test]
    #vis async fn #name() {
      let (databases, _main_temp_file, _session_temp_file) =
        #crate_path::test_utils::create_test_databases_with_config_and_pool_size(
          #configure_sqlite,
          #pool_size,
        )
        .await;
      let main_pool = databases.main().clone();
      let _ = &databases;
      let _ = &main_pool;

      #block
    }
  };

  expanded.into()
}
