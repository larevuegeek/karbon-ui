//! Boucles `{#each}` : itérable parcouru par référence ou consommé, place réservée dans
//! la liste d'enfants d'après sa taille, clé de chaque tour et branche `{:else}`.
use super::{Dest, Generator, R};
use crate::{Result, template::markup::Each};
use proc_macro::TokenStream;

/// `true` pour un chemin d'emplacement : identifiants reliés par `.` (champ ou indice
/// de tuple) ou `::` (module, `static`, `const`), sans appel, indexation ni opérateur.
fn is_place(tokens: &TokenStream) -> bool {
    use proc_macro::{Spacing, TokenTree};
    let tokens = tokens.clone().into_iter().collect::<Vec<_>>();
    let mut i = 0;
    // Après `.`, un segment peut aussi être un indice de tuple (`pair.0`).
    let mut field = false;
    loop {
        match tokens.get(i) {
            Some(TokenTree::Ident(_)) => {}
            Some(TokenTree::Literal(l))
                if field
                    && l.to_string()
                        .bytes()
                        .all(|b| b.is_ascii_digit() || b == b'.') => {}
            _ => return false,
        }
        i += 1;
        match tokens.get(i) {
            None => return true,
            Some(TokenTree::Punct(p)) if p.as_char() == '.' && p.spacing() == Spacing::Alone => {
                field = true;
                i += 1;
            }
            Some(TokenTree::Punct(p)) if p.as_char() == ':' && p.spacing() == Spacing::Joint => {
                if !matches!(tokens.get(i + 1), Some(TokenTree::Punct(q)) if q.as_char() == ':') {
                    return false;
                }
                field = false;
                i += 2;
            }
            _ => return false,
        }
    }
}
impl Generator {
    /// Itérable d'un `{#each}` (énumérée s'il a un indice) et motif de sa boucle.
    fn each_parts(&mut self, each: &Each) -> (String, String) {
        // Un chemin (`self.tracks`, `items`, `data::LISTE`, `crate::X.0`) est parcouru
        // par référence, comme en Svelte ; une expression (`0..n`, `xs.iter().rev()`)
        // est consommée.
        let place = is_place(&each.iterable);
        let iterable = self.out.value(&each.iterable);
        let iterable = if place {
            format!("(&{iterable}).into_iter()")
        } else {
            format!("::core::iter::IntoIterator::into_iter({iterable})")
        };
        let pattern = self.out.value(&each.pattern);
        match &each.index {
            Some(index) => {
                let index = self.out.value(index);
                (
                    format!("::core::iter::Iterator::enumerate({iterable})"),
                    format!("({index},{pattern})"),
                )
            }
            None => (iterable, pattern),
        }
    }
    pub(super) fn each_header(&mut self, each: &Each) -> String {
        let (iterable, pattern) = self.each_parts(each);
        format!("for {pattern} in {iterable}")
    }
    pub(super) fn each(&mut self, each: &Each, dest: Dest, forced: Option<&str>) -> Result<String> {
        // Liste d'enfants : chaque tour ajoute au plus `per_item` éléments, réservés d'un
        // coup d'après la taille connue de l'itérable (un élément fait ~500 octets : la
        // liste ne grandit pas par recopies successives).
        let per_item = if dest.slots {
            0
        } else {
            self.capacity(&each.body)
        };
        // L'itérable est l'objet d'un `match` : ses temporaires restent en vie pendant
        // toute la boucle, comme dans l'expression d'un `for`.
        let (header, close) = if per_item > 0 {
            let (iterable, pattern) = self.each_parts(each);
            (
                format!(
                    "match {iterable}{{__ui_iter=>{{{}.reserve(::core::iter::Iterator::size_hint(&__ui_iter).0*{per_item});for {pattern} in __ui_iter",
                    dest.var
                ),
                "}}",
            )
        } else {
            (self.each_header(each), "")
        };
        let key = self.out.value(&each.key);
        let site = self.next();
        let body = self.children(&each.body, dest, forced)?;
        let mut code = String::new();
        let any = each.otherwise.is_some();
        if any {
            code += "#[allow(unused_assignments)] let mut __ui_any=false;";
        }
        code += &format!(
            "{header}{{{}let __ui_scope={R}::ui::template::keyed(__ui_scope.item({site}),{key});{body}}}{close}",
            if any { "__ui_any=true;" } else { "" }
        );
        if let Some(otherwise) = &each.otherwise {
            code += &format!(
                "if !__ui_any{{{}}}",
                self.children(otherwise, dest, forced)?
            );
        }
        Ok(format!("{{{code}}}"))
    }
}
