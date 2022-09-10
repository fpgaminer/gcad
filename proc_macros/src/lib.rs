use proc_macro::TokenStream;
use quote::{format_ident, quote};


#[proc_macro_attribute]
pub fn ffi_func(_args: TokenStream, input: TokenStream) -> TokenStream {
	let ast = match syn::parse::<syn::ItemFn>(input) {
		Ok(ast) => ast,
		Err(e) => {
			panic!("{}", e);
		},
	};

	let func_ident = ast.sig.ident.clone();
	let mut arg_parsers = Vec::new();
	let mut call_args = Vec::new();

	for (idx, arg) in ast.sig.inputs.iter().enumerate() {
		let ident = match get_argument_ident(arg) {
			Some(ident) => ident,
			None => {
				continue;
			},
		};

		let is_optional = is_argument_optional(arg);
		let arg_ident = format_ident!("arg{}", idx);

		let optional_logic = if is_optional {
			quote! {}
		} else {
			let err_msg = format!("{}: {} is required", func_ident, ident);
			quote! {
				let #arg_ident = #arg_ident.ok_or(anyhow!(#err_msg))?;
			}
		};

		let parser = quote! {
			let mut #arg_ident = args.next().cloned();

			if let Some(arg) = nargs.remove(#ident) {
				#arg_ident = Some(arg);
			}

			#optional_logic
		};

		arg_parsers.push(parser);
		call_args.push(if is_optional {
			let err_msg = format!("Argument {} is not the correct type", idx);
			quote! {
				if let Some(#arg_ident) = #arg_ident { Some(#arg_ident.try_into().map_err(|_| anyhow!(#err_msg))?) } else { None }
			}
		} else {
			let err_msg = format!("Argument {} is not the correct type", idx);
			quote! {
				#arg_ident.try_into().map_err(|_| anyhow!(#err_msg))?
			}
		});
	}

	let ffi_name = format_ident!("{}_ffi", ast.sig.ident);
	let too_many_args_err = format!("{}: too many arguments, expected {}, got {{}}", func_ident, arg_parsers.len());
	let unknown_named_err = format!("{}: unknown named argument {{}}", func_ident);

	let our_func = quote! {
		pub fn #ffi_name(&mut self, args: &[ScriptValue], nargs: &std::collections::HashMap<String, ScriptValue>) -> anyhow::Result<ScriptValue> {
			let arg_len = args.len();
			let mut args = args.into_iter();
			let mut nargs = nargs.clone();
			#(#arg_parsers)*

			if args.next().is_some() {
				bail!(#too_many_args_err, arg_len);
			}

			if let Some(arg) = nargs.into_keys().next() {
				bail!(#unknown_named_err, arg);
			}

			self.#func_ident(#(#call_args),*)
		}

		#ast
	};

	our_func.into()
}


fn is_argument_optional(arg: &syn::FnArg) -> bool {
	if let syn::FnArg::Typed(arg) = arg {
		if let syn::Type::Path(type_path) = &*arg.ty {
			if let Some(first_segment) = type_path.path.segments.first() {
				if first_segment.ident == "Option" {
					return true;
				}
			}
		}
	}

	false
}


fn get_argument_ident(arg: &syn::FnArg) -> Option<String> {
	if let syn::FnArg::Typed(arg) = arg {
		if let syn::Pat::Ident(ident) = &*arg.pat {
			return Some(ident.ident.to_string());
		}
	}

	None
}
