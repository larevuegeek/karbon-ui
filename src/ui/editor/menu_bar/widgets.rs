//! Les dessins de la barre de menus : titre d'un menu, entrée serrée, séparateur.
use super::model::Check;
use crate::{
    application::Action,
    ui::{
        editor::EditorPalette,
        element::{PaintContext, Role, Semantics, SizeF, TextMeasurer, Widget},
        scene::{DrawCommand, PointF, RectF, Scene},
    },
};
use std::{cell::Cell, sync::Arc};

/// Titre d'un menu dans la barre.
pub(super) struct Title {
    pub(super) text: Arc<str>,
    pub(super) open: bool,
    pub(super) action: Action,
    pub(super) palette: EditorPalette,
}

impl Widget for Title {
    fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        let size = text.measure_text(&self.text, self.palette.font_size, false, 4096.)?;
        Ok(SizeF {
            width: size.width.ceil() + 16.,
            height: self.palette.row_height,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        if self.open {
            scene.fill(r, 3., p.selection);
        } else if context.state.hovered || context.state.focused {
            scene.fill(r, 3., p.hover);
        }
        let top = r.top + ((r.bottom - r.top - p.font_size * 1.35) / 2.).max(0.);
        scene.text_single_line(
            RectF::new(
                r.left + 8.,
                top,
                (r.right - r.left - 8.).max(0.),
                p.font_size * 1.4,
            ),
            Arc::clone(&self.text),
            p.font_size,
            false,
            p.text,
        );
    }

    fn action(&self) -> Option<Action> {
        Some(self.action.clone())
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::MenuItem,
            name: &self.text,
            enabled: true,
        }
    }
}

/// Une entrée de menu serrée : marque, libellé, raccourci à droite, flèche de sous-menu.
pub(super) struct Entry {
    pub(super) label: Arc<str>,
    pub(super) shortcut: Arc<str>,
    pub(super) check: Check,
    pub(super) submenu: bool,
    pub(super) action: Option<Action>,
    pub(super) shortcut_width: Cell<f32>,
    pub(super) palette: EditorPalette,
}

/// Marge à gauche du libellé (colonne des marques) et à droite (raccourci, flèche).
const MARK: f32 = 22.;
const ARROW: f32 = 18.;

impl Widget for Entry {
    fn measure(&self, _: SizeF, text: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        let size = self.palette.font_size;
        let label = text.measure_text(&self.label, size, false, 4096.)?.width;
        let shortcut = if self.shortcut.is_empty() {
            0.
        } else {
            text.measure_text(&self.shortcut, size, false, 4096.)?
                .width
                .ceil()
        };
        self.shortcut_width.set(shortcut);
        Ok(SizeF {
            width: MARK + label.ceil() + 24. + shortcut + ARROW,
            height: self.palette.row_height,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let p = &self.palette;
        let r = context.rect;
        let enabled = self.action.is_some();
        let lit =
            enabled && (context.state.hovered || context.state.focused || context.state.pressed);
        if lit {
            scene.fill(r, 3., p.selection);
        }
        let color = if enabled { p.text } else { p.disabled };
        let (cx, cy) = (r.left + 11., (r.top + r.bottom) / 2.);
        match self.check {
            Check::Checked(true) => {
                let at = |x: f32, y: f32| PointF {
                    x: cx + x,
                    y: cy + y,
                };
                for (start, end) in [(at(-4., 0.), at(-1., 3.)), (at(-1., 3.), at(4., -3.))] {
                    scene.commands.push(DrawCommand::Line {
                        start,
                        end,
                        width: 1.5,
                        color: p.accent,
                    });
                }
            }
            Check::Radio(true) => scene.fill(RectF::new(cx - 3., cy - 3., 6., 6.), 3., p.accent),
            _ => {}
        }
        let top = r.top + ((r.bottom - r.top - p.font_size * 1.35) / 2.).max(0.);
        let shortcut = self.shortcut_width.get();
        let label_right = r.right - ARROW - shortcut - 12.;
        scene.text_single_line(
            RectF::new(
                r.left + MARK,
                top,
                (label_right - r.left - MARK).max(0.),
                p.font_size * 1.4,
            ),
            Arc::clone(&self.label),
            p.font_size,
            false,
            color,
        );
        if shortcut > 0. {
            scene.text_single_line(
                RectF::new(
                    r.right - ARROW - shortcut,
                    top,
                    shortcut + 2.,
                    p.font_size * 1.4,
                ),
                Arc::clone(&self.shortcut),
                p.font_size,
                false,
                if enabled { p.muted } else { p.disabled },
            );
        }
        if self.submenu {
            let x = r.right - 12.;
            let points = vec![
                PointF { x, y: cy - 4. },
                PointF { x: x + 5., y: cy },
                PointF { x, y: cy + 4. },
            ];
            scene.commands.push(DrawCommand::Path {
                points,
                closed: true,
                color,
            });
        }
    }

    fn action(&self) -> Option<Action> {
        self.action.clone()
    }

    fn focusable(&self) -> bool {
        self.action.is_some()
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::MenuItem,
            name: &self.label,
            enabled: self.action.is_some(),
        }
    }
}

/// Trait entre deux groupes d'entrées.
pub(super) struct Separator {
    pub(super) palette: EditorPalette,
}

impl Widget for Separator {
    fn measure(&self, _: SizeF, _: &mut dyn TextMeasurer) -> crate::Result<SizeF> {
        Ok(SizeF {
            width: 1.,
            height: 7.,
        })
    }

    fn paint(&self, context: &PaintContext, scene: &mut Scene) {
        let r = context.rect;
        let y = ((r.top + r.bottom) / 2.).floor();
        scene.fill(
            RectF::new(r.left + 6., y, (r.right - r.left - 12.).max(0.), 1.),
            0.,
            self.palette.border,
        );
    }

    fn semantics(&self) -> Semantics<'_> {
        Semantics {
            role: Role::Separator,
            name: "",
            enabled: true,
        }
    }
}
