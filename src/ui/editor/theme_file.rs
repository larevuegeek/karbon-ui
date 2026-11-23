//! Le thème d'un éditeur : les couleurs du kit tirées d'une `EditorPalette` (KUI-049),
//! et un thème lu à l'exécution d'un fichier JSON (KUI-050), pour qu'un utilisateur change
//! ses couleurs et sa taille de texte sans recompiler.
use super::EditorPalette;
use crate::{
    json::Value,
    ui::{
        kit::{KitTheme, Palette},
        theme::Color,
    },
};

/// Les couleurs du kit qui vont avec `p` : boutons, listes et dialogues du kit dans les
/// teintes de l'éditeur.
fn kit_palette(p: &EditorPalette, base: Palette) -> Palette {
    Palette {
        background: p.panel,
        surface: p.field,
        subtle: p.hover,
        border: p.border,
        text: p.text,
        muted: p.muted,
        primary: p.accent,
        ..base
    }
}

impl EditorPalette {
    /// The theme of the kit in the colours of the editor: [`EditorPalette::LIGHT`] for
    /// the light scheme, [`EditorPalette::DARK`] for the dark one, small radii (4) for
    /// dense controls. Give it to [`set_theme`](crate::ui::kit::set_theme); the default
    /// kit theme is unchanged.
    pub fn kit_theme() -> KitTheme {
        KitTheme {
            light: kit_palette(&Self::LIGHT, Palette::LIGHT),
            dark: kit_palette(&Self::DARK, Palette::DARK),
            radius: 4.,
        }
    }

    /// This palette with the values of a theme file in JSON, read at run time: colours
    /// as `"#RRGGBB"` under their field names (`"panel"`, `"field"`, `"hover"`,
    /// `"selection"`, `"text"`, `"muted"`, `"disabled"`, `"accent"`, `"border"`,
    /// `"axis_x"`, `"axis_y"`, `"axis_z"`), and the sizes `"row_height"` (16 to 48) and
    /// `"font_size"` (9 to 24). What the file does not give keeps its value. Fails on
    /// a text that is not a JSON object, a colour that cannot be read, or a size out of
    /// its range, and says which.
    ///
    /// ```
    /// use karbon_ui::ui::{editor::EditorPalette, theme::Color};
    ///
    /// let mine = EditorPalette::DARK
    ///     .with_json(r##"{ "accent": "#FF8800", "font_size": 13 }"##)
    ///     .unwrap();
    /// assert_eq!(mine.accent, Color::hex(0xFF8800));
    /// assert_eq!(mine.font_size, 13.);
    /// assert_eq!(mine.panel, EditorPalette::DARK.panel);
    /// ```
    pub fn with_json(mut self, text: &str) -> Result<Self, String> {
        let Some(Value::Object(members)) = crate::json::parse(text) else {
            return Err("the theme is not a JSON object".into());
        };
        for (key, value) in members {
            self.set(&key, &value)?;
        }
        Ok(self)
    }

    fn set(&mut self, key: &str, value: &Value) -> Result<(), String> {
        let size = |min: f32, max: f32| match value {
            Value::Number(n) if (*n as f32) >= min && (*n as f32) <= max => Ok(*n as f32),
            _ => Err(format!("« {key} » must be a number from {min} to {max}")),
        };
        match key {
            "row_height" => self.row_height = size(16., 48.)?,
            "font_size" => self.font_size = size(9., 24.)?,
            _ => *self.color_mut(key)? = color_of(key, value)?,
        }
        Ok(())
    }

    fn color_mut(&mut self, key: &str) -> Result<&mut Color, String> {
        Ok(match key {
            "panel" => &mut self.panel,
            "field" => &mut self.field,
            "hover" => &mut self.hover,
            "selection" => &mut self.selection,
            "text" => &mut self.text,
            "muted" => &mut self.muted,
            "disabled" => &mut self.disabled,
            "accent" => &mut self.accent,
            "border" => &mut self.border,
            "axis_x" => &mut self.axes[0],
            "axis_y" => &mut self.axes[1],
            "axis_z" => &mut self.axes[2],
            other => return Err(format!("unknown theme entry « {other} »")),
        })
    }
}

/// La couleur `"#RRGGBB"` de l'entrée `key`.
fn color_of(key: &str, value: &Value) -> Result<Color, String> {
    let Value::String(text) = value else {
        return Err(format!("« {key} » must be a colour « #RRGGBB »"));
    };
    let [r, g, b] = super::parse_hex(text)
        .ok_or_else(|| format!("« {key} » : « {text} » is not a colour « #RRGGBB »"))?;
    let byte = |v: f32| (v * 255.).round() as u8;
    Ok(Color::rgb(byte(r), byte(g), byte(b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::editor::contrast;

    #[test]
    fn the_kit_theme_of_the_editor_takes_its_colours_and_keeps_its_contrast() {
        let theme = EditorPalette::kit_theme();
        assert_eq!(theme.dark.primary, EditorPalette::DARK.accent);
        assert_eq!(theme.light.background, EditorPalette::LIGHT.panel);
        assert_eq!(theme.radius, 4.);
        // Le texte et le texte discret se lisent sur les fonds (4,5:1 au moins).
        for p in [theme.dark, theme.light] {
            for background in [p.background, p.surface, p.subtle] {
                assert!(contrast(p.text, background) >= 4.5, "{p:?}");
                assert!(contrast(p.muted, background) >= 4.5, "{p:?}");
            }
        }
        // Le thème du kit par défaut ne change pas.
        assert_eq!(KitTheme::default().radius, 12.);
    }

    #[test]
    fn a_theme_file_changes_what_it_gives_and_says_what_it_cannot_read() {
        let text =
            r##"{ "accent": "#ff8800", "axis_x": "#F00", "row_height": 24, "font_size": 13 }"##;
        let mine = EditorPalette::DARK.with_json(text).unwrap();
        assert_eq!(mine.accent, Color::hex(0xFF8800));
        assert_eq!(mine.axes[0], Color::hex(0xFF0000));
        assert_eq!((mine.row_height, mine.font_size), (24., 13.));
        assert_eq!(mine.text, EditorPalette::DARK.text);
        // Témoins : ce qui ne se lit pas est refusé, et dit.
        let bad = EditorPalette::DARK.with_json(r#"{ "accent": "orange" }"#);
        assert!(bad.unwrap_err().contains("accent"));
        assert!(
            EditorPalette::DARK
                .with_json(r#"{ "font_size": 80 }"#)
                .is_err()
        );
        assert!(
            EditorPalette::DARK
                .with_json(r##"{ "fond": "#000000" }"##)
                .is_err()
        );
        assert!(EditorPalette::DARK.with_json("[1, 2]").is_err());
    }
}
