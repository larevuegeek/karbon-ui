//! Images arrivées dans des [`ImageSlot`] (tâches de fond) : repeintes sans reconstruire
//! la vue ; la mise en page n'est refaite que pour un élément dont la taille dépend de
//! l'image, comme un `<img>` qui finit de charger dans un navigateur.
use super::*;
use crate::ui::template::ImageSlot;

impl UiRuntime {
    /// Des images sont arrivées dans les sources de cet arbre depuis la dernière
    /// vérification : la peinture est demandée, et la mise en page si un élément qui a
    /// mesuré une autre taille dépend de la taille de son image. `true` si une image est
    /// à refaire. Le compteur global évite le parcours quand rien n'est arrivé nulle part.
    pub(super) fn images_arrived(&mut self) -> bool {
        let arrivals = ImageSlot::arrivals();
        if arrivals == self.invalidation.image_arrivals {
            return false;
        }
        self.invalidation.image_arrivals = arrivals;
        let Some(root) = self.root.as_mut() else {
            return false;
        };
        let mut revisions = 0;
        if resized(root, &mut revisions, (false, false)) {
            self.invalidation.relayout = true;
        }
        // Somme des révisions des sources de l'arbre : elle ne bouge pas quand les images
        // arrivent dans une autre fenêtre.
        if revisions == self.invalidation.image_revisions {
            return false;
        }
        self.invalidation.image_revisions = revisions;
        self.invalidation.repaint = true;
        true
    }
}

/// Le sous-arbre contient une image dont la taille compte pour la mise en page et
/// diffère de la taille mesurée ; les conteneurs au-dessus oublient alors leur placement
/// retenu (`LayoutCache::settled`). `revisions` cumule les révisions de ses sources.
/// `parent` : largeur et hauteur du parent fixées, contre lesquelles un pourcentage se
/// résout sans dépendre du contenu (pochette `w-full h-full` d'une case `w-10 h-10`).
fn resized(node: &mut Element, revisions: &mut u64, parent: (bool, bool)) -> bool {
    let definite = |length: Length| matches!(length, Length::Px(_) | Length::Computed(_));
    let fixed = |length: Length, parent: bool| {
        definite(length) || (parent && matches!(length, Length::Percent(_)))
    };
    let slot = node
        .widget
        .as_ref()
        .and_then(|widget| widget.deferred_image());
    *revisions = revisions.wrapping_add(slot.map_or(0, ImageSlot::revision));
    let mut changed = (!fixed(node.style.width, parent.0) || !fixed(node.style.height, parent.1))
        && slot.is_some_and(|slot| {
            crate::ui::template::image_intrinsic(slot.image().as_deref()) != node.intrinsic
        });
    let own = (
        fixed(node.style.width, parent.0),
        fixed(node.style.height, parent.1),
    );
    for child in &mut node.children {
        changed |= resized(child, revisions, own);
    }
    if changed && let Some(cache) = node.layout_cache.as_deref_mut() {
        cache.settled = false;
    }
    changed
}
