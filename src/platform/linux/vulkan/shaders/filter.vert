#version 450
// Passages de filtre (`filter.frag`) : un quad couvrant la zone utile de la cible
// (fenêtre d'affichage), sans tampon de sommets.
void main() {
    vec2 unit = vec2(float(gl_VertexIndex & 1), float(gl_VertexIndex >> 1));
    gl_Position = vec4(unit * 2.0 - 1.0, 0.0, 1.0);
}
