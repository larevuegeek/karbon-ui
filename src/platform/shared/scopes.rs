//! Structure des portées d'une scène validée (clips, effets, filtres, masques,
//! remplissages du texte) : fin d'une portée et portée réduite à un seul élément peint.
use crate::ui::scene::DrawCommand;

/// Index de la commande qui ferme la portée ouverte en `start` (scène validée : portées
/// équilibrées).
pub(in crate::platform) fn scope_end(commands: &[DrawCommand], start: usize) -> usize {
    let mut depth = 0usize;
    for (index, command) in commands.iter().enumerate().skip(start) {
        if opens(command) {
            depth += 1;
        } else if closes(command) {
            depth -= 1;
            if depth == 0 {
                return index;
            }
        }
    }
    commands.len().saturating_sub(1)
}
fn opens(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::PushClip(_)
            | DrawCommand::PushShapeClip { .. }
            | DrawCommand::PushEffect { .. }
            | DrawCommand::PushFilter { .. }
            | DrawCommand::PushMask { .. }
            | DrawCommand::PushTextFill { .. }
    )
}
fn closes(command: &DrawCommand) -> bool {
    matches!(
        command,
        DrawCommand::PopClip
            | DrawCommand::PopShapeClip
            | DrawCommand::PopEffect
            | DrawCommand::PopFilter
            | DrawCommand::PopMask
            | DrawCommand::PopTextFill
    )
}
/// Un seul élément peint (commande simple ou groupe filtré, recomposé en une image) :
/// l'opacité du groupe s'applique directement, sans calque.
pub(in crate::platform) fn single(inner: &[DrawCommand]) -> bool {
    match inner {
        [command] => !opens(command) && !closes(command),
        [DrawCommand::PushFilter { .. }, ..] => scope_end(inner, 0) == inner.len() - 1,
        _ => false,
    }
}
