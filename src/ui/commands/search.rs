//! Recherche floue d'une commande par son nom : les lettres tapées doivent se suivre
//! dans le nom, pas forcément côte à côte (« dupo » trouve « Dupliquer l'objet ») ;
//! les débuts de mots et les lettres collées comptent plus. Sans tenir compte des
//! capitales ni des accents.

/// The letter `c` without accent, in lower case (« É » → « e »).
fn plain(c: char) -> char {
    let lower = c.to_lowercase().next().unwrap_or(c);
    match lower {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' | 'ý' => 'y',
        'œ' => 'o',
        'æ' => 'a',
        other => other,
    }
}

/// How well `query` matches `text`, higher is better; `None` when the letters of
/// `query` (spaces ignored) do not all appear in `text`, in order. Capitals and accents
/// are ignored; the start of a word and letters next to each other count more, an
/// empty query matches everything with `0`.
///
/// ```
/// use karbon_ui::ui::commands::fuzzy_score;
///
/// assert!(fuzzy_score("dup", "Dupliquer").is_some());
/// assert!(fuzzy_score("élé", "Supprimer l'élément").is_some());
/// assert!(fuzzy_score("xyz", "Dupliquer").is_none());
/// // A start of word beats letters scattered in the middle.
/// assert!(fuzzy_score("ob", "Objet") > fuzzy_score("ob", "Robe"));
/// ```
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let wanted: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(plain)
        .collect();
    if wanted.is_empty() {
        return Some(0);
    }
    let mut score = 0;
    let mut next = 0;
    let mut previous_matched = false;
    let mut previous: Option<char> = None;
    for (index, c) in text.chars().enumerate() {
        let start_of_word = previous.is_none_or(|p| !p.is_alphanumeric());
        previous = Some(c);
        if next < wanted.len() && plain(c) == wanted[next] {
            score += 1;
            if start_of_word {
                score += 10;
            }
            if previous_matched {
                score += 6;
            }
            if index == 0 {
                score += 4;
            }
            previous_matched = true;
            next += 1;
        } else {
            previous_matched = false;
            if next > 0 && next < wanted.len() {
                score -= 1;
            }
        }
    }
    (next == wanted.len()).then_some(score)
}

/// The indices of `names` that match `query`, the best first (equal scores keep their
/// order), at most `limit`.
pub fn rank<'a>(query: &str, names: impl IntoIterator<Item = &'a str>, limit: usize) -> Vec<usize> {
    let mut found: Vec<(i32, usize)> = names
        .into_iter()
        .enumerate()
        .filter_map(|(index, name)| fuzzy_score(query, name).map(|score| (score, index)))
        .collect();
    found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    found.truncate(limit);
    found.into_iter().map(|(_, index)| index).collect()
}
