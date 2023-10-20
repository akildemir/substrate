This file is part of a fork of Substrate which has had various changes.

// Copyright (C) Parity Technologies (UK) Ltd.
// Copyright (C) 2022-2023 Luke Parker
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Parse for pallet macro.
//!
//! Parse the module into `Def` struct through `Def::try_from` function.

pub mod call;
pub mod composite;
pub mod config;
pub mod error;
pub mod event;
pub mod extra_constants;
pub mod genesis_build;
pub mod genesis_config;
pub mod helper;
pub mod hooks;
pub mod inherent;
pub mod origin;
pub mod pallet_struct;
pub mod storage;
pub mod type_value;
pub mod validate_unsigned;

use composite::{keyword::CompositeKeyword, CompositeDef};
use frame_support_procedural_tools::generate_crate_access_2018;
use syn::spanned::Spanned;

/// Parsed definition of a pallet.
pub struct Def {
	/// The module items.
	/// (their order must not be modified because they are registered in individual definitions).
	pub item: syn::ItemMod,
	pub config: config::ConfigDef,
	pub pallet_struct: pallet_struct::PalletStructDef,
	pub hooks: Option<hooks::HooksDef>,
	pub call: Option<call::CallDef>,
	pub storages: Vec<storage::StorageDef>,
	pub error: Option<error::ErrorDef>,
	pub event: Option<event::EventDef>,
	pub origin: Option<origin::OriginDef>,
	pub inherent: Option<inherent::InherentDef>,
	pub genesis_config: Option<genesis_config::GenesisConfigDef>,
	pub genesis_build: Option<genesis_build::GenesisBuildDef>,
	pub validate_unsigned: Option<validate_unsigned::ValidateUnsignedDef>,
	pub extra_constants: Option<extra_constants::ExtraConstantsDef>,
	pub composites: Vec<composite::CompositeDef>,
	pub type_values: Vec<type_value::TypeValueDef>,
	pub frame_system: syn::Ident,
	pub frame_support: syn::Ident,
	pub dev_mode: bool,
}

impl Def {
	pub fn try_from(mut item: syn::ItemMod, dev_mode: bool) -> syn::Result<Self> {
		let frame_system = generate_crate_access_2018("frame-system")?;
		let frame_support = generate_crate_access_2018("frame-support")?;

		let item_span = item.span();
		let items = &mut item
			.content
			.as_mut()
			.ok_or_else(|| {
				let msg = "Invalid pallet definition, expected mod to be inlined.";
				syn::Error::new(item_span, msg)
			})?
			.1;

		let mut config = None;
		let mut pallet_struct = None;
		let mut hooks = None;
		let mut call = None;
		let mut error = None;
		let mut event = None;
		let mut origin = None;
		let mut inherent = None;
		let mut genesis_config = None;
		let mut genesis_build = None;
		let mut validate_unsigned = None;
		let mut extra_constants = None;
		let mut storages = vec![];
		let mut type_values = vec![];
		let mut composites: Vec<CompositeDef> = vec![];

		for (index, item) in items.iter_mut().enumerate() {
			let pallet_attr: Option<PalletAttr> = helper::take_first_item_pallet_attr(item)?;

			match pallet_attr {
				Some(PalletAttr::Config(span, with_default)) if config.is_none() =>
					config = Some(config::ConfigDef::try_from(
						&frame_system,
						span,
						index,
						item,
						with_default,
					)?),
				Some(PalletAttr::Pallet(span)) if pallet_struct.is_none() => {
					let p = pallet_struct::PalletStructDef::try_from(span, index, item)?;
					pallet_struct = Some(p);
				},
				Some(PalletAttr::Hooks(span)) if hooks.is_none() => {
					let m = hooks::HooksDef::try_from(span, index, item)?;
					hooks = Some(m);
				},
				Some(PalletAttr::RuntimeCall(cw, span)) if call.is_none() =>
					call = Some(call::CallDef::try_from(span, index, item, dev_mode, cw)?),
				Some(PalletAttr::Error(span)) if error.is_none() =>
					error = Some(error::ErrorDef::try_from(span, index, item)?),
				Some(PalletAttr::RuntimeEvent(span)) if event.is_none() =>
					event = Some(event::EventDef::try_from(span, index, item)?),
				Some(PalletAttr::GenesisConfig(_)) if genesis_config.is_none() => {
					let g = genesis_config::GenesisConfigDef::try_from(index, item)?;
					genesis_config = Some(g);
				},
				Some(PalletAttr::GenesisBuild(span)) if genesis_build.is_none() => {
					let g = genesis_build::GenesisBuildDef::try_from(span, index, item)?;
					genesis_build = Some(g);
				},
				Some(PalletAttr::RuntimeOrigin(_)) if origin.is_none() =>
					origin = Some(origin::OriginDef::try_from(index, item)?),
				Some(PalletAttr::Inherent(_)) if inherent.is_none() =>
					inherent = Some(inherent::InherentDef::try_from(index, item)?),
				Some(PalletAttr::Storage(span)) =>
					storages.push(storage::StorageDef::try_from(span, index, item, dev_mode)?),
				Some(PalletAttr::ValidateUnsigned(_)) if validate_unsigned.is_none() => {
					let v = validate_unsigned::ValidateUnsignedDef::try_from(index, item)?;
					validate_unsigned = Some(v);
				},
				Some(PalletAttr::TypeValue(span)) =>
					type_values.push(type_value::TypeValueDef::try_from(span, index, item)?),
				Some(PalletAttr::ExtraConstants(_)) =>
					extra_constants =
						Some(extra_constants::ExtraConstantsDef::try_from(index, item)?),
				Some(PalletAttr::Composite(span)) => {
					let composite =
						composite::CompositeDef::try_from(span, index, &frame_support, item)?;
					if composites.iter().any(|def| {
						match (&def.composite_keyword, &composite.composite_keyword) {
							(
								CompositeKeyword::FreezeReason(_),
								CompositeKeyword::FreezeReason(_),
							) |
							(CompositeKeyword::HoldReason(_), CompositeKeyword::HoldReason(_)) |
							(CompositeKeyword::LockId(_), CompositeKeyword::LockId(_)) |
							(
								CompositeKeyword::SlashReason(_),
								CompositeKeyword::SlashReason(_),
							) => true,
							_ => false,
						}
					}) {
						let msg = format!(
							"Invalid duplicated `{}` definition",
							composite.composite_keyword
						);
						return Err(syn::Error::new(composite.composite_keyword.span(), &msg))
					}
					composites.push(composite);
				},
				Some(attr) => {
					let msg = "Invalid duplicated attribute";
					return Err(syn::Error::new(attr.span(), msg))
				},
				None => (),
			}
		}

		if genesis_config.is_some() != genesis_build.is_some() {
			let msg = format!(
				"`#[pallet::genesis_config]` and `#[pallet::genesis_build]` attributes must be \
				either both used or both not used, instead genesis_config is {} and genesis_build \
				is {}",
				genesis_config.as_ref().map_or("unused", |_| "used"),
				genesis_build.as_ref().map_or("unused", |_| "used"),
			);
			return Err(syn::Error::new(item_span, msg))
		}

		let def = Def {
			item,
			config: config
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[pallet::config]`"))?,
			pallet_struct: pallet_struct
				.ok_or_else(|| syn::Error::new(item_span, "Missing `#[pallet::pallet]`"))?,
			hooks,
			call,
			extra_constants,
			genesis_config,
			genesis_build,
			validate_unsigned,
			error,
			event,
			origin,
			inherent,
			storages,
			composites,
			type_values,
			frame_system,
			frame_support,
			dev_mode,
		};

		def.check_instance_usage()?;
		def.check_event_usage()?;

		Ok(def)
	}

	/// Check that usage of trait `Event` is consistent with the definition, i.e. it is declared
	/// and trait defines type RuntimeEvent, or not declared and no trait associated type.
	fn check_event_usage(&self) -> syn::Result<()> {
		match (self.config.has_event_type, self.event.is_some()) {
			(true, false) => {
				let msg = "Invalid usage of RuntimeEvent, `Config` contains associated type `RuntimeEvent`, \
					but enum `Event` is not declared (i.e. no use of `#[pallet::event]`). \
					Note that type `RuntimeEvent` in trait is reserved to work alongside pallet event.";
				Err(syn::Error::new(proc_macro2::Span::call_site(), msg))
			},
			(false, true) => {
				let msg = "Invalid usage of RuntimeEvent, `Config` contains no associated type \
					`RuntimeEvent`, but enum `Event` is declared (in use of `#[pallet::event]`). \
					An RuntimeEvent associated type must be declare on trait `Config`.";
				Err(syn::Error::new(proc_macro2::Span::call_site(), msg))
			},
			_ => Ok(()),
		}
	}

	/// Check that usage of trait `Config` is consistent with the definition, i.e. it is used with
	/// instance iff it is defined with instance.
	fn check_instance_usage(&self) -> syn::Result<()> {
		let mut instances = vec![];
		instances.extend_from_slice(&self.pallet_struct.instances[..]);
		instances.extend(&mut self.storages.iter().flat_map(|s| s.instances.clone()));
		if let Some(call) = &self.call {
			instances.extend_from_slice(&call.instances[..]);
		}
		if let Some(hooks) = &self.hooks {
			instances.extend_from_slice(&hooks.instances[..]);
		}
		if let Some(event) = &self.event {
			instances.extend_from_slice(&event.instances[..]);
		}
		if let Some(error) = &self.error {
			instances.extend_from_slice(&error.instances[..]);
		}
		if let Some(inherent) = &self.inherent {
			instances.extend_from_slice(&inherent.instances[..]);
		}
		if let Some(origin) = &self.origin {
			instances.extend_from_slice(&origin.instances[..]);
		}
		if let Some(genesis_config) = &self.genesis_config {
			instances.extend_from_slice(&genesis_config.instances[..]);
		}
		if let Some(genesis_build) = &self.genesis_build {
			genesis_build.instances.as_ref().map(|i| instances.extend_from_slice(&i));
		}
		if let Some(extra_constants) = &self.extra_constants {
			instances.extend_from_slice(&extra_constants.instances[..]);
		}

		let mut errors = instances.into_iter().filter_map(|instances| {
			if instances.has_instance == self.config.has_instance {
				return None
			}
			let msg = if self.config.has_instance {
				"Invalid generic declaration, trait is defined with instance but generic use none"
			} else {
				"Invalid generic declaration, trait is defined without instance but generic use \
						some"
			};
			Some(syn::Error::new(instances.span, msg))
		});

		if let Some(mut first_error) = errors.next() {
			for error in errors {
				first_error.combine(error)
			}
			Err(first_error)
		} else {
			Ok(())
		}
	}

	/// Depending on if pallet is instantiable:
	/// * either `T: Config`
	/// * or `T: Config<I>, I: 'static`
	pub fn type_impl_generics(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		if self.config.has_instance {
			quote::quote_spanned!(span => T: Config<I>, I: 'static)
		} else {
			quote::quote_spanned!(span => T: Config)
		}
	}

	/// Depending on if pallet is instantiable:
	/// * either `T: Config`
	/// * or `T: Config<I>, I: 'static = ()`
	pub fn type_decl_bounded_generics(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		if self.config.has_instance {
			quote::quote_spanned!(span => T: Config<I>, I: 'static = ())
		} else {
			quote::quote_spanned!(span => T: Config)
		}
	}

	/// Depending on if pallet is instantiable:
	/// * either `T`
	/// * or `T, I = ()`
	pub fn type_decl_generics(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		if self.config.has_instance {
			quote::quote_spanned!(span => T, I = ())
		} else {
			quote::quote_spanned!(span => T)
		}
	}

	/// Depending on if pallet is instantiable:
	/// * either ``
	/// * or `<I>`
	/// to be used when using pallet trait `Config`
	pub fn trait_use_generics(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		if self.config.has_instance {
			quote::quote_spanned!(span => <I>)
		} else {
			quote::quote_spanned!(span => )
		}
	}

	/// Depending on if pallet is instantiable:
	/// * either `T`
	/// * or `T, I`
	pub fn type_use_generics(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		if self.config.has_instance {
			quote::quote_spanned!(span => T, I)
		} else {
			quote::quote_spanned!(span => T)
		}
	}
}

/// Some generic kind for type which can be not generic, or generic over config,
/// or generic over config and instance, but not generic only over instance.
pub enum GenericKind {
	None,
	Config,
	ConfigAndInstance,
}

impl GenericKind {
	/// Return Err if it is only generics over instance but not over config.
	pub fn from_gens(has_config: bool, has_instance: bool) -> Result<Self, ()> {
		match (has_config, has_instance) {
			(false, false) => Ok(GenericKind::None),
			(true, false) => Ok(GenericKind::Config),
			(true, true) => Ok(GenericKind::ConfigAndInstance),
			(false, true) => Err(()),
		}
	}

	/// Return the generic to be used when using the type.
	///
	/// Depending on its definition it can be: ``, `T` or `T, I`
	pub fn type_use_gen(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		match self {
			GenericKind::None => quote::quote!(),
			GenericKind::Config => quote::quote_spanned!(span => T),
			GenericKind::ConfigAndInstance => quote::quote_spanned!(span => T, I),
		}
	}

	/// Return the generic to be used in `impl<..>` when implementing on the type.
	pub fn type_impl_gen(&self, span: proc_macro2::Span) -> proc_macro2::TokenStream {
		match self {
			GenericKind::None => quote::quote!(),
			GenericKind::Config => quote::quote_spanned!(span => T: Config),
			GenericKind::ConfigAndInstance => {
				quote::quote_spanned!(span => T: Config<I>, I: 'static)
			},
		}
	}

	/// Return whereas the type has some generic.
	pub fn is_generic(&self) -> bool {
		match self {
			GenericKind::None => false,
			GenericKind::Config | GenericKind::ConfigAndInstance => true,
		}
	}
}

/// List of additional token to be used for parsing.
mod keyword {
	syn::custom_keyword!(origin);
	syn::custom_keyword!(call);
	syn::custom_keyword!(weight);
	syn::custom_keyword!(event);
	syn::custom_keyword!(config);
	syn::custom_keyword!(with_default);
	syn::custom_keyword!(hooks);
	syn::custom_keyword!(inherent);
	syn::custom_keyword!(error);
	syn::custom_keyword!(storage);
	syn::custom_keyword!(genesis_build);
	syn::custom_keyword!(genesis_config);
	syn::custom_keyword!(validate_unsigned);
	syn::custom_keyword!(type_value);
	syn::custom_keyword!(pallet);
	syn::custom_keyword!(generate_store);
	syn::custom_keyword!(Store);
	syn::custom_keyword!(extra_constants);
	syn::custom_keyword!(composite_enum);
}

/// Parse attributes for item in pallet module
/// syntax must be `pallet::` (e.g. `#[pallet::config]`)
enum PalletAttr {
	Config(proc_macro2::Span, bool),
	Pallet(proc_macro2::Span),
	Hooks(proc_macro2::Span),
	/// A `#[pallet::call]` with optional attributes to specialize the behaviour.
	///
	/// # Attributes
	///
	/// Each attribute `attr` can take the form of `#[pallet::call(attr = …)]` or
	/// `#[pallet::call(attr(…))]`. The possible attributes are:
	///
	/// ## `weight`
	///
	/// Can be used to reduce the repetitive weight annotation in the trivial case. It accepts one
	/// argument that is expected to be an implementation of the `WeightInfo` or something that
	/// behaves syntactically equivalent. This allows to annotate a `WeightInfo` for all the calls.
	/// Now each call does not need to specify its own `#[pallet::weight]` but can instead use the
	/// one from the `#[pallet::call]` definition. So instead of having to write it on each call:
	///
	/// ```ignore
	/// #[pallet::call]
	/// impl<T: Config> Pallet<T> {
	///     #[pallet::weight(T::WeightInfo::create())]
	///     pub fn create(
	/// ```
	/// you can now omit it on the call itself, if the name of the weigh function matches the call:
	///
	/// ```ignore
	/// #[pallet::call(weight = <T as crate::Config>::WeightInfo)]
	/// impl<T: Config> Pallet<T> {
	///     pub fn create(
	/// ```
	///
	/// It is possible to use this syntax together with instantiated pallets by using `Config<I>`
	/// instead.
	///
	/// ### Dev Mode
	///
	/// Normally the `dev_mode` sets all weights of calls without a `#[pallet::weight]` annotation
	/// to zero. Now when there is a `weight` attribute on the `#[pallet::call]`, then that is used
	/// instead of the zero weight. So to say: it works together with `dev_mode`.
	RuntimeCall(Option<InheritedCallWeightAttr>, proc_macro2::Span),
	Error(proc_macro2::Span),
	RuntimeEvent(proc_macro2::Span),
	RuntimeOrigin(proc_macro2::Span),
	Inherent(proc_macro2::Span),
	Storage(proc_macro2::Span),
	GenesisConfig(proc_macro2::Span),
	GenesisBuild(proc_macro2::Span),
	ValidateUnsigned(proc_macro2::Span),
	TypeValue(proc_macro2::Span),
	ExtraConstants(proc_macro2::Span),
	Composite(proc_macro2::Span),
}

impl PalletAttr {
	fn span(&self) -> proc_macro2::Span {
		match self {
			Self::Config(span, _) => *span,
			Self::Pallet(span) => *span,
			Self::Hooks(span) => *span,
			Self::RuntimeCall(_, span) => *span,
			Self::Error(span) => *span,
			Self::RuntimeEvent(span) => *span,
			Self::RuntimeOrigin(span) => *span,
			Self::Inherent(span) => *span,
			Self::Storage(span) => *span,
			Self::GenesisConfig(span) => *span,
			Self::GenesisBuild(span) => *span,
			Self::ValidateUnsigned(span) => *span,
			Self::TypeValue(span) => *span,
			Self::ExtraConstants(span) => *span,
			Self::Composite(span) => *span,
		}
	}
}

impl syn::parse::Parse for PalletAttr {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		input.parse::<syn::Token![#]>()?;
		let content;
		syn::bracketed!(content in input);
		content.parse::<keyword::pallet>()?;
		content.parse::<syn::Token![::]>()?;

		let lookahead = content.lookahead1();
		if lookahead.peek(keyword::config) {
			let span = content.parse::<keyword::config>()?.span();
			let with_default = content.peek(syn::token::Paren);
			if with_default {
				let inside_config;
				let _paren = syn::parenthesized!(inside_config in content);
				inside_config.parse::<keyword::with_default>()?;
			}
			Ok(PalletAttr::Config(span, with_default))
		} else if lookahead.peek(keyword::pallet) {
			Ok(PalletAttr::Pallet(content.parse::<keyword::pallet>()?.span()))
		} else if lookahead.peek(keyword::hooks) {
			Ok(PalletAttr::Hooks(content.parse::<keyword::hooks>()?.span()))
		} else if lookahead.peek(keyword::call) {
			let span = content.parse::<keyword::call>().expect("peeked").span();
			let attr = match content.is_empty() {
				true => None,
				false => Some(InheritedCallWeightAttr::parse(&content)?),
			};
			Ok(PalletAttr::RuntimeCall(attr, span))
		} else if lookahead.peek(keyword::error) {
			Ok(PalletAttr::Error(content.parse::<keyword::error>()?.span()))
		} else if lookahead.peek(keyword::event) {
			Ok(PalletAttr::RuntimeEvent(content.parse::<keyword::event>()?.span()))
		} else if lookahead.peek(keyword::origin) {
			Ok(PalletAttr::RuntimeOrigin(content.parse::<keyword::origin>()?.span()))
		} else if lookahead.peek(keyword::inherent) {
			Ok(PalletAttr::Inherent(content.parse::<keyword::inherent>()?.span()))
		} else if lookahead.peek(keyword::storage) {
			Ok(PalletAttr::Storage(content.parse::<keyword::storage>()?.span()))
		} else if lookahead.peek(keyword::genesis_config) {
			Ok(PalletAttr::GenesisConfig(content.parse::<keyword::genesis_config>()?.span()))
		} else if lookahead.peek(keyword::genesis_build) {
			Ok(PalletAttr::GenesisBuild(content.parse::<keyword::genesis_build>()?.span()))
		} else if lookahead.peek(keyword::validate_unsigned) {
			Ok(PalletAttr::ValidateUnsigned(content.parse::<keyword::validate_unsigned>()?.span()))
		} else if lookahead.peek(keyword::type_value) {
			Ok(PalletAttr::TypeValue(content.parse::<keyword::type_value>()?.span()))
		} else if lookahead.peek(keyword::extra_constants) {
			Ok(PalletAttr::ExtraConstants(content.parse::<keyword::extra_constants>()?.span()))
		} else if lookahead.peek(keyword::composite_enum) {
			Ok(PalletAttr::Composite(content.parse::<keyword::composite_enum>()?.span()))
		} else {
			Err(lookahead.error())
		}
	}
}

/// The optional weight annotation on a `#[pallet::call]` like `#[pallet::call(weight($type))]`.
#[derive(Clone)]
pub struct InheritedCallWeightAttr {
	pub typename: syn::Type,
	pub span: proc_macro2::Span,
}

impl syn::parse::Parse for InheritedCallWeightAttr {
	// Parses `(weight($type))` or `(weight = $type)`.
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let content;
		syn::parenthesized!(content in input);
		content.parse::<keyword::weight>()?;
		let lookahead = content.lookahead1();

		let buffer = if lookahead.peek(syn::token::Paren) {
			let inner;
			syn::parenthesized!(inner in content);
			inner
		} else if lookahead.peek(syn::Token![=]) {
			content.parse::<syn::Token![=]>().expect("peeked");
			content
		} else {
			return Err(lookahead.error())
		};

		Ok(Self { typename: buffer.parse()?, span: input.span() })
	}
}
