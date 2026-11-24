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
        let mut walk = Walk {
            revisions: &mut revisions,
            measured: &mut self.invalidation.image_measures,
            layouts: self.invalidation.stats.layouts,
        };
        if resized(root, &mut walk, (false, false)) {
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
///
/// Un élément dont la mesure ne suit pas son image (une vue 3D qui prend la place qu'on
/// lui donne) mesurerait sans fin une autre taille que celle de l'image : une fois une
/// mise en page faite avec une image de cette taille, la même taille ne la redemande
/// plus (`Walk::measured`). Le résultat de la mise en page est le même.
fn resized(node: &mut Element, walk: &mut Walk<'_>, parent: (bool, bool)) -> bool {
    let definite = |length: Length| matches!(length, Length::Px(_) | Length::Computed(_));
    let fixed = |length: Length, parent: bool| {
        definite(length) || (parent && matches!(length, Length::Percent(_)))
    };
    let slot = node
        .widget
        .as_ref()
        .and_then(|widget| widget.deferred_image());
    *walk.revisions = walk
        .revisions
        .wrapping_add(slot.map_or(0, ImageSlot::revision));
    let mut changed = (!fixed(node.style.width, parent.0) || !fixed(node.style.height, parent.1))
        && slot.is_some_and(|slot| {
            let image = crate::ui::template::image_intrinsic(slot.image().as_deref());
            image != node.intrinsic && walk.must_measure(node.id, node.intrinsic, image)
        });
    let own = (
        fixed(node.style.width, parent.0),
        fixed(node.style.height, parent.1),
    );
    for child in &mut node.children {
        changed |= resized(child, walk, own);
    }
    if changed && let Some(cache) = node.layout_cache.as_deref_mut() {
        cache.settled = false;
    }
    changed
}

/// Parcours des images arrivées.
struct Walk<'a> {
    /// Somme des révisions des sources.
    revisions: &'a mut u64,
    /// Par élément : sa taille mesurée et celle de son image quand une mise en page a
    /// été demandée pour elle, et le compte des mises en page à ce moment.
    measured: &'a mut crate::ui::core::hash::FastMap<ElementId, (SizeF, SizeF, u64)>,
    /// Mises en page faites jusqu'ici.
    layouts: u64,
}

impl Walk<'_> {
    /// Une image de taille `image` dans l'élément `id`, qui a mesuré `intrinsic` :
    /// faut-il refaire la mise en page ? Non si une mise en page a déjà eu lieu avec une
    /// image de cette taille et a laissé la même mesure.
    fn must_measure(&mut self, id: ElementId, intrinsic: SizeF, image: SizeF) -> bool {
        let settled = self.measured.get(&id).is_some_and(|&(before, seen, at)| {
            before == intrinsic && seen == image && at < self.layouts
        });
        if !settled {
            self.measured.insert(id, (intrinsic, image, self.layouts));
        }
        !settled
    }
}
