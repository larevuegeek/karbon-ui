//! Clics multiples dans un champ : double clic = mot sans espaces, triple clic = ligne,
//! clic simple = caret et sélection repliée ; le glisser qui suit garde la sélection du
//! mot. Mesureur déterministe : 0,5 em par caractère (les abscisses visent le milieu
//! des caractères, quelle que soit la taille de police par défaut).
use super::*;

fn click(f: &mut Fixture, p: PointF) -> EventResult {
    let down = f.event(UiEvent::PointerDown(p));
    f.event(UiEvent::PointerUp(p));
    down
}

#[test]
fn double_click_selects_the_word_and_triple_click_the_line() {
    let mut f = Fixture::new("ab cd");
    let b = f.state().bounds;
    let at = |x: f32| PointF {
        x: b.left + x,
        y: (b.top + b.bottom) / 2.,
    };
    // Un clic : caret ; le second au même endroit : le mot « ab » ; le troisième : tout.
    click(&mut f, at(3.));
    assert_eq!(f.state().selection, 0..0);
    click(&mut f, at(3.));
    assert_eq!(f.state().selection, 0..2);
    click(&mut f, at(3.));
    assert_eq!(f.state().selection, 0..5);
    // Un quatrième appui recommence : simple clic, sélection repliée.
    click(&mut f, at(3.));
    assert_eq!(f.state().selection, 0..0);
    // Ailleurs (plus de 4 unités) : nouveau compte ; à la fin de « cd », le mot « cd ».
    click(&mut f, at(33.));
    click(&mut f, at(33.));
    assert_eq!(f.state().selection, 3..5);
    // Le glisser après un double clic (nouvelle série, ailleurs) garde le mot.
    click(&mut f, at(3.));
    f.event(UiEvent::PointerDown(at(3.)));
    assert_eq!(f.state().selection, 0..2);
    f.event(UiEvent::PointerMove(at(33.)));
    assert_eq!(f.state().selection, 0..2);
    assert!(f.event(UiEvent::PointerUp(at(33.))).release);
    // Sur la suite d'espaces : la suite d'espaces.
    f.text(" ");
    f.edit(EditCommand::SelectAll);
    f.text("ab   cd");
    click(&mut f, at(21.));
    click(&mut f, at(21.));
    assert_eq!(f.state().selection, 2..5);
    // Un clic simple ailleurs replie la sélection, Maj+clic l'étend sans compter.
    click(&mut f, at(3.));
    assert_eq!(f.state().selection, 0..0);
    f.event(UiEvent::PointerDownExtended(at(55.)));
    f.event(UiEvent::PointerUp(at(55.)));
    assert_eq!(f.state().selection, 0..7);
    assert!(f.ui.scene().validate().is_ok());
}
