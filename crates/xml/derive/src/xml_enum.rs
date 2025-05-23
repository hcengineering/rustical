use super::{attrs::EnumAttrs, Variant};
use crate::attrs::VariantAttrs;
use core::panic;
use darling::{FromDeriveInput, FromVariant};
use quote::quote;
use syn::{DataEnum, DeriveInput};

pub struct Enum {
    attrs: EnumAttrs,
    variants: Vec<Variant>,
    ident: syn::Ident,
    generics: syn::Generics,
}

impl Enum {
    fn impl_de_untagged(&self) -> proc_macro2::TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.ident;

        let variant_branches = self
            .variants
            .iter()
            .filter_map(|variant| variant.untagged_branch());

        quote! {
            impl #impl_generics ::rustical_xml::XmlDeserialize for #name #type_generics #where_clause {
                fn deserialize<R: ::std::io::BufRead>(
                    reader: &mut quick_xml::NsReader<R>,
                    start: &quick_xml::events::BytesStart,
                    empty: bool
                ) -> Result<Self, rustical_xml::XmlError> {
                    #(#variant_branches);*

                    Err(rustical_xml::XmlError::InvalidVariant("could not match".to_owned()))
                }
            }
        }
    }

    fn impl_de_tagged(&self) -> proc_macro2::TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let name = &self.ident;

        let variant_branches = self.variants.iter().filter_map(Variant::tagged_branch);

        quote! {
            impl #impl_generics ::rustical_xml::XmlDeserialize for #name #type_generics #where_clause {
                fn deserialize<R: std::io::BufRead>(
                    reader: &mut quick_xml::NsReader<R>,
                    start: &quick_xml::events::BytesStart,
                    empty: bool
                ) -> Result<Self, rustical_xml::XmlError> {
                    let (_ns, name) = reader.resolve_element(start.name());

                    match name.as_ref() {
                        #(#variant_branches),*
                        name => {
                            // Handle invalid variant name
                            Err(rustical_xml::XmlError::InvalidVariant(String::from_utf8_lossy(name).to_string()))
                        }
                    }
                }
            }
        }
    }

    pub fn impl_de(&self) -> proc_macro2::TokenStream {
        match self.attrs.untagged.is_present() {
            true => self.impl_de_untagged(),
            false => self.impl_de_tagged(),
        }
    }

    pub fn parse(input: &DeriveInput, data: &DataEnum) -> Self {
        let attrs = EnumAttrs::from_derive_input(input).unwrap();

        Self {
            variants: data
                .variants
                .iter()
                .map(|variant| Variant {
                    attrs: VariantAttrs::from_variant(variant).unwrap(),
                    variant: variant.to_owned(),
                })
                .collect(),
            attrs,
            ident: input.ident.to_owned(),
            generics: input.generics.to_owned(),
        }
    }

    pub fn impl_se(&self) -> proc_macro2::TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;
        let enum_untagged = self.attrs.untagged.is_present();
        let variant_serializers = self.variants.iter().map(Variant::se_branch);

        quote! {
            impl #impl_generics ::rustical_xml::XmlSerialize for #ident #type_generics #where_clause {
                fn serialize<W: ::std::io::Write>(
                    &self,
                    ns: Option<::quick_xml::name::Namespace>,
                    tag: Option<&[u8]>,
                    namespaces: &::std::collections::HashMap<::quick_xml::name::Namespace, &[u8]>,
                    writer: &mut ::quick_xml::Writer<W>
                ) -> ::std::io::Result<()> {
                    use ::quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

                    let prefix = ns
                        .map(|ns| namespaces.get(&ns))
                        .unwrap_or(None)
                        .map(|prefix| [*prefix, b":"].concat());
                    let has_prefix = prefix.is_some();
                    let tagname = tag.map(|tag| [&prefix.unwrap_or_default(), tag].concat());
                    let qname = tagname.as_ref().map(|tagname| ::quick_xml::name::QName(tagname));

                    const enum_untagged: bool = #enum_untagged;

                    if let Some(qname) = &qname {
                        let mut bytes_start = BytesStart::from(qname.to_owned());
                        if !has_prefix {
                            if let Some(ns) = &ns {
                                bytes_start.push_attribute((b"xmlns".as_ref(), ns.as_ref()));
                            }
                        }
                        writer.write_event(Event::Start(bytes_start))?;
                    }

                    #(#variant_serializers);*

                    if let Some(qname) = &qname {
                        writer.write_event(Event::End(BytesEnd::from(qname.to_owned())))?;
                    }
                    Ok(())
                }

                fn attributes<'a>(&self) -> Option<Vec<::quick_xml::events::attributes::Attribute<'a>>> {
                    None
                }
            }
        }
    }

    pub fn impl_xml_document(&self) -> proc_macro2::TokenStream {
        if self.attrs.untagged.is_present() {
            panic!("XmlDocument only supported for untagged enums");
        }
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;

        quote! {
            impl #impl_generics ::rustical_xml::XmlDocument for #ident #type_generics #where_clause {
                fn parse<R: ::std::io::BufRead>(mut reader: ::quick_xml::NsReader<R>) -> Result<Self, ::rustical_xml::XmlError>
                where
                    Self: ::rustical_xml::XmlDeserialize
                {
                    use ::quick_xml::events::Event;

                    let mut buf = Vec::new();
                    loop {
                        let event = reader.read_event_into(&mut buf)?;
                        let empty = matches!(event, Event::Empty(_));

                        match event {
                            Event::Start(start) | Event::Empty(start) => {
                                return <Self as ::rustical_xml::XmlDeserialize>::deserialize(&mut reader, &start, empty);
                            }
                            Event::Eof => return Err(::rustical_xml::XmlError::Eof),
                            Event::Text(bytes_text) => {
                                return Err(::rustical_xml::XmlError::UnsupportedEvent("Text"));
                            }
                            Event::CData(cdata) => {
                                return Err(::rustical_xml::XmlError::UnsupportedEvent("CDATA"));
                            }
                            Event::Decl(_) => { /* <?xml ... ?> ignore this */ }
                            Event::Comment(_) => { /* ignore */ }
                            Event::DocType(_) => { /* ignore */ }
                            Event::PI(_) => {
                                return Err(::rustical_xml::XmlError::UnsupportedEvent("Processing instruction"));
                            }
                            Event::End(end) => {
                                unreachable!("Premature end of xml document, should be handled by quick_xml");
                            }
                        };
                    }
                }
            }
        }
    }

    pub fn impl_enum_variants(&self) -> proc_macro2::TokenStream {
        let (impl_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let ident = &self.ident;

        if self.attrs.untagged.is_present() {
            let untagged_variants = self.variants.iter().map(|variant| {
                let ty = &variant.deserializer_type();
                quote! { #ty::variant_names() }
            });
            quote! {
                impl #impl_generics ::rustical_xml::EnumVariants for #ident #type_generics #where_clause {
                    const TAGGED_VARIANTS: &'static [(Option<::quick_xml::name::Namespace<'static>>, &'static str)] = &[];

                    fn variant_names() -> Vec<(Option<::quick_xml::name::Namespace<'static>>, &'static str)> {
                        [
                            #(#untagged_variants),*
                        ].concat()
                    }
                }
            }
        } else {
            let tagged_variants = self.variants.iter().map(|variant| {
                let ns = match &variant.attrs.common.ns {
                    Some(ns) => quote! { Some(#ns) },
                    None => quote! { None },
                };
                let b_xml_name = variant.xml_name().value();
                let xml_name = String::from_utf8_lossy(&b_xml_name);
                quote! {(#ns, #xml_name)}
            });

            quote! {
                impl #impl_generics ::rustical_xml::EnumVariants for #ident #type_generics #where_clause {
                    const TAGGED_VARIANTS: &'static [(Option<::quick_xml::name::Namespace<'static>>, &'static str)] = &[
                        #(#tagged_variants),*
                    ];

                    fn variant_names() -> Vec<(Option<::quick_xml::name::Namespace<'static>>, &'static str)> {
                        [Self::TAGGED_VARIANTS,].concat()
                    }
                }
            }
        }
    }

    pub fn impl_enum_unit_variants(&self) -> proc_macro2::TokenStream {
        let unit_enum_ident = self
            .attrs
            .unit_variants_ident
            .as_ref()
            .expect("unit_variants_ident no set");
        let ident = &self.ident;

        if self.attrs.untagged.is_present() {
            let variant_branches: Vec<_> = self
                .variants
                .iter()
                .map(|variant| {
                    let variant_type = variant.deserializer_type();
                    let variant_ident = &variant.variant.ident;
                    quote! {
                        #variant_ident (<#variant_type as ::rustical_xml::EnumUnitVariants>::UnitVariants)
                    }
                })
                .collect();

            let variant_idents: Vec<_> = self
                .variants
                .iter()
                .map(|variant| &variant.variant.ident)
                .collect();

            let unit_to_output_branches = variant_idents.iter().map(|variant_ident| {
                quote! { #unit_enum_ident::#variant_ident(val) => val.into() }
            });

            let str_to_unit_branches = self.variants.iter().map(|variant| {
                let variant_type = variant.deserializer_type();
                let variant_ident = &variant.variant.ident;
                quote! {
                    if let Ok(name) = <#variant_type as ::rustical_xml::EnumUnitVariants>::UnitVariants::from_str(val) {
                        return Ok(Self::#variant_ident(name))
                    }
                }
            });

            let from_enum_to_unit_branches = variant_idents.iter().map(|variant_ident| {
                quote! { #ident::#variant_ident(val) => #unit_enum_ident::#variant_ident(val.into()) }
            });

            quote! {
                #[derive(Clone, Debug, PartialEq)]
                pub enum #unit_enum_ident {
                    #(#variant_branches),*
                }

                impl ::rustical_xml::EnumUnitVariants for #ident {
                    type UnitVariants = #unit_enum_ident;
                }

                impl From<#unit_enum_ident> for (Option<::quick_xml::name::Namespace<'static>>, &'static str) {
                    fn from(val: #unit_enum_ident) -> Self {
                        match val {
                            #(#unit_to_output_branches),*
                        }
                    }
                }

                 impl From<#ident> for #unit_enum_ident {
                     fn from(val: #ident) -> Self {
                         match val {
                             #(#from_enum_to_unit_branches),*
                         }
                     }
                 }

                 impl ::std::str::FromStr for #unit_enum_ident {
                     type Err = ::rustical_xml::FromStrError;

                     fn from_str(val: &str) -> Result<Self, Self::Err> {
                        #(#str_to_unit_branches);*
                        Err(::rustical_xml::FromStrError)
                     }
                 }
            }
        } else {
            let tagged_variants: Vec<_> = self
                .variants
                .iter()
                .filter(|variant| !variant.attrs.other.is_present())
                .collect();

            let variant_outputs: Vec<_> = tagged_variants
                .iter()
                .map(|variant| {
                    let ns = match &variant.attrs.common.ns {
                        Some(ns) => quote! { Some(#ns) },
                        None => quote! { None },
                    };
                    let b_xml_name = variant.xml_name().value();
                    let xml_name = String::from_utf8_lossy(&b_xml_name);
                    quote! {(#ns, #xml_name)}
                })
                .collect();

            let variant_idents: Vec<_> = tagged_variants
                .iter()
                .map(|variant| &variant.variant.ident)
                .collect();

            let unit_to_output_branches =
                variant_idents
                    .iter()
                    .zip(&variant_outputs)
                    .map(|(variant_ident, out)| {
                        quote! { #unit_enum_ident::#variant_ident => #out }
                    });

            let from_enum_to_unit_branches = variant_idents.iter().map(|variant_ident| {
                quote! { #ident::#variant_ident { .. } => #unit_enum_ident::#variant_ident }
            });

            let str_to_unit_branches = tagged_variants.iter().map(|variant| {
                let variant_ident = &variant.variant.ident;
                let b_xml_name = variant.xml_name().value();
                let xml_name = String::from_utf8_lossy(&b_xml_name);
                quote! { #xml_name => Ok(#unit_enum_ident::#variant_ident) }
            });

            quote! {
                #[derive(Clone, Debug, PartialEq)]
                pub enum #unit_enum_ident {
                    #(#variant_idents),*
                }


                impl ::rustical_xml::EnumUnitVariants for #ident {
                    type UnitVariants = #unit_enum_ident;
                }

                impl From<#unit_enum_ident> for (Option<::quick_xml::name::Namespace<'static>>, &'static str) {
                    fn from(val: #unit_enum_ident) -> Self {
                        match val {
                            #(#unit_to_output_branches),*
                        }
                    }
                }

                impl From<#ident> for #unit_enum_ident {
                    fn from(val: #ident) -> Self {
                        match val {
                            #(#from_enum_to_unit_branches),*
                        }
                    }
                }

                impl ::std::str::FromStr for #unit_enum_ident {
                    type Err = ::rustical_xml::FromStrError;

                    fn from_str(val: &str) -> Result<Self, Self::Err> {
                        match val {
                            #(#str_to_unit_branches),*,
                            _ => Err(::rustical_xml::FromStrError)
                        }
                    }
                }
            }
        }
    }
}
