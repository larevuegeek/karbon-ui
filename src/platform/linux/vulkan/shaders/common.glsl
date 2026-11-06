// Structures partagées par les shaders du renderer Vulkan de karbon-ui : miroir exact
// de `platform/shared/gpu.rs` (std430, champs de 16 octets, sans remplissage),
// comme les structures de `macos/metal/shaders.metal`.
struct Instance {
    vec4 quad;   // quad dans le repère local (gauche, haut, droite, bas)
    vec4 m;      // repère local → pixels : m11 m12 m21 m22
    vec4 t;      // dx, dy, taille d'un pixel dans le repère local, opacité
    vec4 shape;  // boîte extérieure, ou segment (x0 y0 x1 y1)
    vec4 radii;  // rayons extérieurs (hg hd bd bg) ; segment : demi-épaisseur en x
    vec4 inner;  // boîte intérieure d'un anneau (trait)
    vec4 iradii; // rayons intérieurs
    vec4 color;  // couleur non prémultipliée
    vec4 paint;  // dégradé linéaire (x0 y0 x1 y1) ou radial (cx cy rx ry)
    vec4 uv;     // coordonnées de texture aux coins du quad
    vec4 clip;   // découpe rectangulaire en pixels de la cible
    uvec4 kind;  // forme, peinture, premier arrêt, nombre d'arrêts
    ivec4 extra; // maillon de découpe générale (-1 : aucun) ; y = 1 : peinture
                 // multipliée par la texture (glyphes sous `background-clip: text`)
};
struct Stop {
    vec4 color;    // non prémultipliée
    vec4 position; // x
};
// Découpe arrondie : m, t.xy = pixels → repère de la découpe ; t.z = taille d'un pixel.
// Masque en dégradé (`mask-image`) : rect = extrémités du dégradé dans ce repère.
struct ClipNode {
    vec4 m;
    vec4 t;
    vec4 rect;
    vec4 radii;
    ivec4 link; // x : maillon parent (-1 : fin) ; y : 0 découpe, 1 masque ; z, w : arrêts
};
layout(std430, set = 0, binding = 0) readonly buffer Instances { Instance instances[]; };
// Coin de la cible dans les pixels de la fenêtre, et taille de la texture cible.
layout(push_constant) uniform Frame {
    vec2 origin;
    vec2 size;
} frame;
