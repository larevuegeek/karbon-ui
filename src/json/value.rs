//! Valeur JSON lue : les objets gardent l'ordre du texte (une clé répétée : le lecteur
//! garde la dernière).

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

// Accès des tests : les lecteurs du crate (`storage`, `intl`) parcourent l'arbre eux-mêmes.
#[cfg(test)]
impl Value {
    /// Membre `key` d'un objet ; `None` pour un autre type.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    /// Texte d'une chaîne.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }
}
