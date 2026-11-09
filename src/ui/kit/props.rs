//! Valeurs d'attributs communes : textes, listes de textes, choix.
use crate::ui::template::IntoProp;

/// Text attribute: a literal, a `String` or a borrowed string.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Text(pub(super) String);

impl IntoProp<Text> for &str {
    fn into_prop(self) -> Text {
        Text(self.to_owned())
    }
}
impl IntoProp<Text> for String {
    fn into_prop(self) -> Text {
        Text(self)
    }
}
impl IntoProp<Text> for &String {
    fn into_prop(self) -> Text {
        Text(self.clone())
    }
}

/// List of texts: `["Name", "Size"]`, or a `Vec<String>`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Texts(pub(super) Vec<String>);

impl<const N: usize> IntoProp<Texts> for [&str; N] {
    fn into_prop(self) -> Texts {
        Texts(self.iter().map(|s| (*s).to_owned()).collect())
    }
}
impl IntoProp<Texts> for &[&str] {
    fn into_prop(self) -> Texts {
        Texts(self.iter().map(|s| (*s).to_owned()).collect())
    }
}
impl IntoProp<Texts> for Vec<String> {
    fn into_prop(self) -> Texts {
        Texts(self)
    }
}

/// Choices of a list: `["Low", "High"]` (values 0, 1…), or `[(10, "Low"), (20, "High")]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options(pub(super) Vec<(u64, String)>);

impl<const N: usize> IntoProp<Options> for [&str; N] {
    fn into_prop(self) -> Options {
        Options(
            self.iter()
                .enumerate()
                .map(|(i, s)| (i as u64, (*s).to_owned()))
                .collect(),
        )
    }
}
impl<const N: usize> IntoProp<Options> for [(u64, &str); N] {
    fn into_prop(self) -> Options {
        Options(self.iter().map(|(v, s)| (*v, (*s).to_owned())).collect())
    }
}
impl IntoProp<Options> for Vec<String> {
    fn into_prop(self) -> Options {
        Options(
            self.into_iter()
                .enumerate()
                .map(|(i, s)| (i as u64, s))
                .collect(),
        )
    }
}
impl IntoProp<Options> for Vec<(u64, String)> {
    fn into_prop(self) -> Options {
        Options(self)
    }
}

/// Number attribute accepted as `f32` from the usual numeric types.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Number(pub(super) f32);

macro_rules! number {
    ($($t:ty),*) => {$(
        impl IntoProp<Number> for $t {
            fn into_prop(self) -> Number {
                Number(self as f32)
            }
        }
    )*};
}
number!(i32, u32, i64, u64, usize, f32, f64);
