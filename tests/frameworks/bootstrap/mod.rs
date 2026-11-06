//! Bootstrap 5.3.3 (`tests/fixtures/bootstrap`) compilé en mode framework : composants et
//! variables, contrôles dessinés par la feuille, et la page de `examples/bootstrap_demo`
//! pilotée comme un utilisateur.
mod components;
mod controls;
mod demo;

use karbon_ui::ui::prelude::*;
css_file!(mod bs, "../../fixtures/bootstrap/bootstrap.css", framework);
