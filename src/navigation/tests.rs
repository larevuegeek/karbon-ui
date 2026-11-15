//! Déplacements dans l'historique, comme dans un onglet de navigateur.
use super::History;

#[derive(Debug, Clone, PartialEq)]
enum Route {
    Home,
    Playlists,
    Radios,
    Stats,
    Settings(u8),
}

#[test]
fn goes_back_and_forward() {
    let mut h = History::new(Route::Home);
    assert!(h.push(Route::Playlists));
    assert!(!h.push(Route::Playlists));
    assert!(h.push(Route::Radios));
    assert!(h.back());
    assert_eq!(*h.current(), Route::Playlists);
    assert!(h.forward());
    assert_eq!(*h.current(), Route::Radios);
    assert!(h.back() && h.back());
    assert_eq!(*h.current(), Route::Home);
    assert!(!h.back() && !h.can_go_back());
    assert_eq!(h.ahead(), [Route::Radios, Route::Playlists]);
    assert!(h.push(Route::Stats));
    assert!(!h.forward(), "a new page drops the ones ahead");
    assert_eq!(h.behind(), [Route::Home]);
}

#[test]
fn back_past_leaves_a_group_of_pages() {
    let settings = |r: &Route| matches!(r, Route::Settings(_));
    let mut h = History::new(Route::Home);
    h.push(Route::Playlists);
    h.push(Route::Settings(0));
    h.push(Route::Settings(1));
    h.push(Route::Settings(2));
    assert!(h.back_past(settings, Route::Home));
    assert_eq!(*h.current(), Route::Playlists);
    assert_eq!(h.ahead(), [Route::Settings(2)]);
    assert!(h.back());
    assert_eq!(*h.current(), Route::Home);
    // Rien d'autre derrière : la page de repli.
    let mut h = History::new(Route::Settings(0));
    assert!(h.back_past(settings, Route::Home));
    assert_eq!(*h.current(), Route::Home);
}

#[test]
fn replace_keeps_the_entries() {
    let mut h = History::new(Route::Home);
    h.push(Route::Settings(0));
    h.replace(Route::Settings(1));
    assert_eq!(*h.current(), Route::Settings(1));
    assert!(h.back());
    assert_eq!(*h.current(), Route::Home);
}
