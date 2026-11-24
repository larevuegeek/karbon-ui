//! Peinture gardée par widget (KUI-056) : un widget qui donne une clé de peinture
//! (`Widget::paint_key`) n'est repeint que si sa clé, sa boîte, son état visuel ou le
//! thème ont changé ; sinon ses commandes de l'image précédente sont recopiées telles
//! quelles. La scène produite est la même, octet pour octet.
use crate::ui::{
    core::hash::FastMap,
    element::{ElementId, PaintContext, Widget},
    scene::{DrawCommand, RectF, Scene},
    theme::Theme,
};

/// Commandes d'un widget peint, et ce dont elles dépendent.
struct Kept {
    key: u64,
    rect: RectF,
    /// Survolé, appuyé, focalisé.
    state: (bool, bool, bool),
    theme: Theme,
    commands: Vec<DrawCommand>,
    /// Dernière peinture de la scène qui s'en est servie.
    generation: u64,
}

/// Les commandes gardées des widgets qui ont une clé de peinture, par élément.
#[derive(Default)]
pub(crate) struct KeptPaints {
    entries: FastMap<ElementId, Kept>,
    generation: u64,
    /// Peintures de widgets évitées depuis le début (compteur de mesure).
    pub(crate) reused: u64,
}

impl KeptPaints {
    /// Début d'une peinture de la scène.
    pub(crate) fn begin(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Fin d'une peinture : les widgets qui n'ont pas été peints sont oubliés.
    pub(crate) fn end(&mut self) {
        let generation = self.generation;
        self.entries.retain(|_, kept| kept.generation == generation);
    }

    /// Peint `widget` dans `scene`, ou recopie ses commandes gardées.
    pub(crate) fn paint(
        &mut self,
        id: ElementId,
        widget: &dyn Widget,
        context: &PaintContext,
        scene: &mut Scene,
    ) {
        let Some(key) = widget.paint_key() else {
            return widget.paint(context, scene);
        };
        let state = (
            context.state.hovered,
            context.state.pressed,
            context.state.focused,
        );
        let generation = self.generation;
        if let Some(kept) = self.entries.get_mut(&id)
            && kept.key == key
            && kept.rect == context.rect
            && kept.state == state
            && kept.theme == context.theme
        {
            kept.generation = generation;
            scene.commands.extend_from_slice(&kept.commands);
            self.reused += 1;
            return;
        }
        let start = scene.commands.len();
        widget.paint(context, scene);
        self.entries.insert(
            id,
            Kept {
                key,
                rect: context.rect,
                state,
                theme: context.theme,
                commands: scene.commands[start..].to_vec(),
                generation,
            },
        );
    }
}
