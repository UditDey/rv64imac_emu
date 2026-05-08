use proc_macro::TokenStream;
use quote::quote;
use syn::token::{Colon, Comma};
use syn::{
    Attribute, Generics, Ident, LitInt, Token, Visibility,
    parse::{Parse, ParseStream, Result},
};

// AST representing a single-bit or range field
enum FieldKind {
    Single(u64),                    // Single bit at given position
    Range { start: u64, end: u64 }, // Inclusive bit range
}

// Definition of a field: visibility, name, and bitkind
struct FieldDef {
    name: Ident,
    kind: FieldKind,
}

// Definition of the bitfield spec: attrs, visibility, name, generic repr, and fields
struct BitfieldDef {
    attrs: Vec<Attribute>, // e.g. #[derive(...)]
    vis: Visibility,
    name: Ident,
    ty: syn::Type,
    fields: Vec<FieldDef>,
}

// Parse one FieldDef from DSL: `pub field: 0`, `pub range: 3..=5`, etc.
impl Parse for FieldDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let _vis: Visibility = input.parse()?; // e.g. `pub`
        let name: Ident = input.parse()?; // field name
        input.parse::<Colon>()?; // `:`
        let lookahead = input.lookahead1();
        let kind = if lookahead.peek(LitInt) {
            let start_lit: LitInt = input.parse()?;
            let start = start_lit.base10_parse()?;
            if input.peek(Token![..]) {
                input.parse::<Token![..]>()?;
                input.parse::<Token![=]>()?;
                let end_lit: LitInt = input.parse()?;
                let end = end_lit.base10_parse()?;
                FieldKind::Range { start, end }
            } else {
                FieldKind::Single(start)
            }
        } else {
            return Err(lookahead.error());
        };
        if input.peek(Comma) {
            input.parse::<Comma>()?;
        }
        Ok(FieldDef { name, kind })
    }
}

// Parse entire bitfield spec from DSL, capturing outer attributes
impl Parse for BitfieldDef {
    fn parse(input: ParseStream) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?; // capture #[derive]
        let vis: Visibility = input.parse()?; // `pub`
        input.parse::<Token![struct]>()?; // `struct`
        let name: Ident = input.parse()?; // struct name
        let generics: Generics = input.parse()?; // `<u64>`
        let content;
        syn::braced!(content in input); // `{ ... }`

        // Underlying repr type from generic parameter
        let ty = if let Some(arg) = generics.params.first() {
            if let syn::GenericParam::Type(tp) = arg {
                syn::parse_str::<syn::Type>(&tp.ident.to_string())
                    .expect("Expected concrete type in <> for bitfield repr")
            } else {
                panic!("Unexpected non-type generic parameter");
            }
        } else {
            panic!("Missing generic parameter for repr type");
        };

        let mut fields = Vec::new();
        while !content.is_empty() {
            fields.push(content.parse()?);
        }
        Ok(BitfieldDef {
            attrs,
            vis,
            name,
            ty,
            fields,
        })
    }
}

// Function-like macro entrypoint: parses DSL and emits tuple struct + methods
#[proc_macro]
pub fn bitfield(input: TokenStream) -> TokenStream {
    // Parse the input spec
    let def = syn::parse_macro_input!(input as BitfieldDef);
    let attrs = &def.attrs;
    let vis = &def.vis;
    let name = &def.name;
    let repr_ty = &def.ty;

    // Compute mask for all fields
    let total_mask: u128 = def
        .fields
        .iter()
        .map(|field| match field.kind {
            FieldKind::Single(pos) => 1u128 << pos,
            FieldKind::Range { start, end } => ((1u128 << (end - start + 1)) - 1) << start,
        })
        .fold(0, |acc, m| acc | m);
    let mask_lit = syn::LitInt::new(&total_mask.to_string(), name.span());

    // Build getters/setters
    let mut methods = Vec::new();
    for field in &def.fields {
        let fname = &field.name;
        let fname_str = fname.to_string();
        let getter = Ident::new(&fname_str, fname.span());
        let setter = Ident::new(&format!("set_{}", fname_str), fname.span());
        match field.kind {
            FieldKind::Single(bit) => {
                let idx = syn::Index::from(bit as usize);
                methods.push(quote! {
                    pub fn #getter(&self) -> bool {
                        ((self.0 >> #idx) & 1) != 0
                    }
                    pub fn #setter(&mut self, val: bool) {
                        if val { self.0 |= 1 << #idx; }
                        else     { self.0 &= !(1 << #idx); }
                    }
                });
            }
            FieldKind::Range { start, end } => {
                let width = end - start + 1;
                let field_mask = ((1u128 << width) - 1) << start;
                let field_mask_lit = syn::LitInt::new(&field_mask.to_string(), fname.span());
                let shift = syn::LitInt::new(&start.to_string(), fname.span());
                methods.push(quote! {
                    pub fn #getter(&self) -> #repr_ty {
                        ((self.0 & #field_mask_lit) >> #shift) as #repr_ty
                    }
                    pub fn #setter(&mut self, val: #repr_ty) {
                        let v = ((val as u128 & ((1u128 << #width) - 1)) << #shift) as #repr_ty;
                        let cleared = (self.0 as u128 & !(#field_mask_lit as u128)) as #repr_ty;
                        self.0 = (cleared as u128 | v as u128) as #repr_ty;
                    }
                });
            }
        }
    }

    // Emit tuple struct and impl with from_bits/to_bits
    let expanded = quote! {
        #(#attrs)*
        #vis struct #name(#repr_ty);
        impl #name {
            pub fn from_bits(bits: #repr_ty) -> Self {
                Self((bits as u128 & #mask_lit) as #repr_ty)
            }
            pub const VALID_MASK: #repr_ty = #mask_lit;
            pub fn as_bits(&self) -> #repr_ty { self.0 }
            #(#methods)*
        }
    };

    TokenStream::from(expanded)
}
