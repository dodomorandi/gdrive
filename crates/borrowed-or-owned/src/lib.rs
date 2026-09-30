//! A smart reference that is either a borrowed value or an owned one.
//!
//! [`MaybeOwned`] is the runtime dependency of the code generator that emits one model per
//! storage style (owned, borrowed, and cow variants) for Google API schemas. The two halves of
//! the pair — the borrowed view and the owned model — are separate generated types, which is
//! exactly the shape [`std::borrow::Cow`] cannot express.
//!
//! # Why not `Cow`
//!
//! `Cow<'a, B>` is parameterized by a single type `B`, and its owned half is `<B as
//! ToOwned>::Owned`. Two properties of that trait make it a poor fit for a generated model
//! pair.
//!
//! First, `ToOwned` fixes exactly one owned type per borrowed type, and the trait requires
//! `type Owned: Borrow<Self>`: the owned value must be able to hand the borrowed view back
//! through `Borrow::borrow(&self) -> &B`. Primitives satisfy this — `String: Borrow<str>`,
//! which is exactly why `Cow<'a, str>` covers a single string field. A generated model pair
//! cannot: the borrowed view spells its fields as `&'a str` borrowed from the caller's decode
//! buffer, so no `&borrowed::User<'a>` exists inside `&owned::User` for that lifetime and
//! `owned::User: Borrow<borrowed::User<'a>>` does not compile. Second, the pairing would have
//! to be declared in the borrowed model's own `ToOwned` impl — one per generated model, and
//! regenerated whenever the owned model's fields change.
//!
//! [`MaybeOwned`] keeps the two types unrelated and relates them with a pair of traits modelled on
//! [`std::borrow::ToOwned`] and [`std::borrow::Borrow`]: [`ToOwnedModel`] copies a borrowed model
//! into an owned one, and [`BorrowModel`] builds a borrowed view out of an owned one. The
//! generator instantiates the smart reference as `MaybeOwned<'a, borrowed::User<'a>,
//! owned::User>`.
//!
//! # Why not `mown` or `boow`
//!
//! The `mown` and `boow` crates offer the same borrowed-or-owned idea and were evaluated
//! alongside `Cow`. Both require the owned type to borrow back as the borrowed type
//! (`O: Borrow<B>`), which is what lets them hand out a `&B` from the owned variant through
//! `Deref` — the very requirement that [`ToOwned`] imposes, and the one a generated owned
//! model cannot satisfy without a self-referential owner, since the owned struct cannot lend
//! out a `&'a str` that outlives the borrow of the struct itself. [`BorrowModel`] asks for the
//! view by value instead, which a generated model can always provide.
//!
//! # Usage
//!
//! ```
//! use borrowed_or_owned::{MaybeOwned, ToOwnedModel};
//!
//! // Stands in for the generated `borrowed::User<'a>` and `owned::User` pair.
//! struct User<'a> {
//!     name: &'a str,
//! }
//!
//! #[derive(Clone)]
//! struct OwnedUser {
//!     name: String,
//! }
//!
//! impl ToOwnedModel for User<'_> {
//!     type Owned = OwnedUser;
//!
//!     fn to_owned_model(&self) -> OwnedUser {
//!         OwnedUser { name: self.name.to_owned() }
//!     }
//! }
//!
//! let view = User { name: "gdrive" };
//! let borrowed = MaybeOwned::<User<'_>, OwnedUser>::borrowed(&view);
//! let owned = MaybeOwned::<User<'_>, OwnedUser>::owned(OwnedUser { name: String::from("gdrive") });
//!
//! assert!(borrowed.is_borrowed());
//! assert!(owned.is_owned());
//! assert_eq!(borrowed.to_owned().name, "gdrive");
//! assert_eq!(owned.into_owned().name, "gdrive");
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Copies a borrowed model into an owned one.
///
/// This is the generated counterpart of [`std::borrow::ToOwned`]. It exists separately because
/// the owned model is a different generated type, so the association is expressed in the other
/// direction: `borrowed::User<'a>` knows how to build an `owned::User`, and the result carries no
/// lifetime because the copy owns everything it reads.
///
/// The conversion deep-copies, so the owned model never aliases the borrowed data.
pub trait ToOwnedModel {
    /// The owned model built by [`ToOwnedModel::to_owned_model`].
    type Owned: Sized;

    /// Copies this borrowed model into its owned counterpart.
    fn to_owned_model(&self) -> Self::Owned;
}

/// Lends a borrowed view of an owned model.
///
/// This is the generated counterpart of [`std::borrow::Borrow`], with one difference that a
/// generated model forces: the view is a different type, so it is returned by value rather than
/// by reference. The view borrows from `self`, which is why the associated type takes the
/// lifetime of the borrow.
///
/// ```
/// use borrowed_or_owned::BorrowModel;
///
/// struct Name(String);
/// struct NameView<'a>(&'a str);
///
/// impl BorrowModel for Name {
///     type Borrowed<'a> = NameView<'a> where Self: 'a;
///
///     fn as_borrowed(&self) -> NameView<'_> {
///         NameView(self.0.as_str())
///     }
/// }
///
/// let name = Name(String::from("gdrive"));
/// assert_eq!(name.as_borrowed().0, "gdrive");
/// ```
pub trait BorrowModel {
    /// The borrowed view produced by [`BorrowModel::as_borrowed`].
    type Borrowed<'a>: Sized
    where
        Self: 'a;

    /// Builds a borrowed view of this owned model.
    fn as_borrowed(&self) -> Self::Borrowed<'_>;
}

/// A value that is either borrowed for `'a` or owned outright.
///
/// `B` is the borrowed view type and may be unsized (`str` or `[u8]` for a raw buffer, or a
/// sized generated borrowed model such as `borrowed::User<'a>`); `O` is the owned type. A
/// generated crate instantiates it per schema field as
/// `MaybeOwned<'a, borrowed::User<'a>, owned::User>`, where the two types come from separate
/// generated modules and are only related by an `O: From<&'a B>` impl.
///
/// There is deliberately no `From<O>` impl: it would collide with core's reflexive
/// `From<T> for T`. Use the [`MaybeOwned::owned`] constructor instead.
#[derive(Debug, PartialEq, Eq, Hash)]
#[must_use = "a `MaybeOwned` is almost always meant to be read, so drop it explicitly if not"]
pub enum MaybeOwned<'a, B: ?Sized, O> {
    /// Borrows a value of the view type for `'a`, without copying it.
    Borrowed(&'a B),
    /// Holds the owned value.
    Owned(O),
}

impl<'a, B: ?Sized, O> MaybeOwned<'a, B, O> {
    /// Wraps a borrowed value.
    pub const fn borrowed(value: &'a B) -> Self {
        Self::Borrowed(value)
    }

    /// Wraps an owned value.
    pub const fn owned(value: O) -> Self {
        Self::Owned(value)
    }

    /// Returns `true` when this value is borrowed.
    #[must_use]
    pub fn is_borrowed(&self) -> bool {
        matches!(self, Self::Borrowed(_))
    }

    /// Returns `true` when this value is owned.
    #[must_use]
    pub fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }

    /// Returns the borrowed value when this is borrowed, and [`None`] when it is owned.
    ///
    /// The returned reference is the one this value was built from, not a copy of it.
    #[must_use]
    pub fn as_borrowed(&self) -> Option<&'a B> {
        match self {
            Self::Borrowed(value) => Some(value),
            Self::Owned(_) => None,
        }
    }

    /// Returns the owned value when this is owned, and [`None`] when it is borrowed.
    #[must_use]
    pub fn as_owned(&self) -> Option<&O> {
        match self {
            Self::Borrowed(_) => None,
            Self::Owned(value) => Some(value),
        }
    }

    /// Returns the borrowed value when this is borrowed, and [`None`] when it is owned.
    #[must_use]
    pub fn into_borrowed(self) -> Option<&'a B> {
        match self {
            Self::Borrowed(value) => Some(value),
            Self::Owned(_) => None,
        }
    }

    /// Returns the owned value, copying a borrowed one through [`ToOwnedModel`].
    ///
    /// The conversion deep-copies, so the result never aliases the borrowed data.
    pub fn into_owned(self) -> O
    where
        B: ToOwnedModel<Owned = O>,
    {
        match self {
            Self::Borrowed(value) => value.to_owned_model(),
            Self::Owned(value) => value,
        }
    }

    /// Returns a copy of the owned value, leaving this value in place.
    ///
    /// A borrowed value is copied through [`ToOwnedModel`]; an owned one is cloned.
    #[must_use]
    pub fn to_owned(&self) -> O
    where
        B: ToOwnedModel<Owned = O>,
        O: Clone,
    {
        match self {
            Self::Borrowed(value) => value.to_owned_model(),
            Self::Owned(value) => value.clone(),
        }
    }

    /// Returns a borrowed view of the owned value, and [`None`] when this value is borrowed.
    ///
    /// Use this together with [`MaybeOwned::as_borrowed`] to read a value without caring which
    /// variant holds it. Unlike [`MaybeOwned::as_borrowed`], the view is built, so it costs a
    /// shallow copy of the model's references.
    #[must_use]
    pub fn as_view(&self) -> Option<<O as BorrowModel>::Borrowed<'_>>
    where
        O: BorrowModel,
    {
        match self {
            Self::Borrowed(_) => None,
            Self::Owned(value) => Some(value.as_borrowed()),
        }
    }
}

/// Borrows a value into the [`Borrowed`](MaybeOwned::Borrowed) variant.
impl<'a, B: ?Sized, O> From<&'a B> for MaybeOwned<'a, B, O> {
    fn from(value: &'a B) -> Self {
        Self::Borrowed(value)
    }
}

/// Clones the borrowed or the owned half, without requiring `B: Clone`.
///
/// `&B` is [`Copy`] for every `B`, so a borrowed value is copied rather than cloned.
impl<B: ?Sized, O: Clone> Clone for MaybeOwned<'_, B, O> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(value) => Self::Borrowed(value),
            Self::Owned(value) => Self::Owned(value.clone()),
        }
    }
}

impl<B: ?Sized, O: Copy> Copy for MaybeOwned<'_, B, O> {}

#[cfg(test)]
mod tests {
    use super::{BorrowModel, MaybeOwned, ToOwnedModel};

    /// Stands in for a generated `borrowed::User<'a>`.
    struct View<'a>(&'a str);

    /// Stands in for a generated `owned::User`.
    #[derive(Clone, Debug, PartialEq)]
    struct Owned(String);

    impl ToOwnedModel for View<'_> {
        type Owned = Owned;

        fn to_owned_model(&self) -> Owned {
            Owned(self.0.to_owned())
        }
    }

    impl BorrowModel for Owned {
        type Borrowed<'a>
            = View<'a>
        where
            Self: 'a;

        fn as_borrowed(&self) -> View<'_> {
            View(self.0.as_str())
        }
    }

    #[test]
    fn borrowed_arm_yields_the_original_reference() {
        let original = String::from("x");
        let smart = MaybeOwned::<str, String>::borrowed(&original);

        assert!(smart.is_borrowed());
        assert!(!smart.is_owned());
        assert!(std::ptr::eq(
            smart.as_borrowed().unwrap(),
            original.as_str()
        ));
        assert!(smart.as_owned().is_none());
    }

    #[test]
    fn into_borrowed_yields_the_reference_or_nothing() {
        let original = String::from("x");
        let borrowed = MaybeOwned::<str, String>::borrowed(&original);
        let owned = MaybeOwned::<str, String>::owned(String::from("x"));

        assert!(std::ptr::eq(
            borrowed.into_borrowed().unwrap(),
            original.as_str()
        ));
        assert!(owned.into_borrowed().is_none());
    }

    #[test]
    fn borrowed_arm_converts_to_a_deep_copy() {
        let original = Owned(String::from("x"));
        let anchor = original.0.as_ptr();
        let view = View(original.0.as_str());
        let borrowed = MaybeOwned::borrowed(&view);

        let into_owned = borrowed.clone().into_owned();
        let to_owned = borrowed.to_owned();

        assert_eq!(into_owned.0, "x");
        assert!(!std::ptr::eq(into_owned.0.as_ptr(), anchor));
        assert_eq!(to_owned.0, "x");
        assert!(!std::ptr::eq(to_owned.0.as_ptr(), anchor));
    }

    #[test]
    fn owned_arm_yields_and_returns_the_same_value() {
        let original = String::from("x");
        let anchor = original.as_ptr();
        let smart = MaybeOwned::<str, String>::owned(original);

        assert!(smart.is_owned());
        assert!(!smart.is_borrowed());
        assert!(smart.as_borrowed().is_none());
        assert!(std::ptr::eq(smart.as_owned().unwrap().as_ptr(), anchor));
    }

    #[test]
    fn owned_arm_to_owned_returns_an_equal_copy() {
        let smart = MaybeOwned::<View<'_>, Owned>::owned(Owned(String::from("x")));
        let copy = smart.to_owned();

        assert_eq!(copy.0, "x");
        assert!(!std::ptr::eq(
            copy.0.as_ptr(),
            smart.as_owned().unwrap().0.as_ptr()
        ));
    }

    #[test]
    fn owned_arm_into_owned_keeps_the_same_value() {
        let smart = MaybeOwned::<View<'_>, Owned>::owned(Owned(String::from("x")));
        let anchor = smart.as_owned().unwrap().0.as_ptr();

        assert!(std::ptr::eq(smart.into_owned().0.as_ptr(), anchor));
    }

    #[test]
    fn owned_arm_lends_a_view_that_borrows_its_buffer() {
        let original = Owned(String::from("x"));
        let anchor = original.0.as_ptr();
        let smart = MaybeOwned::<View<'_>, Owned>::owned(original);

        let view = smart.as_view().unwrap();

        assert_eq!(view.0, "x");
        assert!(std::ptr::eq(view.0.as_ptr(), anchor));
    }

    #[test]
    fn borrowed_arm_has_no_view() {
        let original = Owned(String::from("x"));
        let view = View(original.0.as_str());
        let smart = MaybeOwned::<View<'_>, Owned>::borrowed(&view);

        assert!(smart.as_view().is_none());
        assert!(std::ptr::eq(
            smart.as_borrowed().unwrap().0.as_ptr(),
            original.0.as_ptr()
        ));
    }

    #[test]
    fn unsized_views_are_cloneable() {
        let borrowed = MaybeOwned::<str, String>::borrowed("borrowed");
        let owned = MaybeOwned::<str, String>::owned(String::from("owned"));

        let borrowed_clone = borrowed.clone();
        let owned_clone = owned.clone();

        assert_eq!(borrowed_clone, borrowed);
        assert_eq!(owned_clone, owned);
        assert!(std::ptr::eq(
            borrowed_clone.as_borrowed().unwrap(),
            "borrowed"
        ));
        assert!(!std::ptr::eq(
            owned_clone.as_owned().unwrap().as_ptr(),
            owned.as_owned().unwrap().as_ptr()
        ));
    }

    #[test]
    fn equality_is_per_variant() {
        fn assert_eq_impl<T: Eq>() {}

        assert_eq_impl::<MaybeOwned<str, String>>();

        let borrowed = MaybeOwned::<str, String>::borrowed("x");
        let other_borrowed = MaybeOwned::<str, String>::borrowed("x");
        let owned = MaybeOwned::<str, String>::owned(String::from("x"));

        assert_eq!(borrowed, other_borrowed);
        assert_eq!(owned, MaybeOwned::<str, String>::owned(String::from("x")));
        assert_ne!(borrowed, owned);
        assert_ne!(owned, borrowed);
        assert_ne!(borrowed, MaybeOwned::<str, String>::borrowed("y"));
        assert_ne!(owned, MaybeOwned::<str, String>::owned(String::from("y")));
    }

    #[test]
    fn debug_names_the_variant() {
        let borrowed = MaybeOwned::<str, String>::borrowed("x");
        let owned = MaybeOwned::<str, String>::owned(String::from("x"));

        assert!(format!("{borrowed:?}").contains("Borrowed"));
        assert!(format!("{owned:?}").contains("Owned"));
    }
}
