#!/bin/sh
# Régénère les shaders SPIR-V du renderer Vulkan (versionnés à côté de leurs sources).
# Outil des développeurs seulement : `glslangValidator` (paquet glslang-tools) ; la
# compilation du crate n'en a pas besoin. À relancer après chaque modification d'un
# shader GLSL, puis commiter les `.spv`.
set -e
cd "$(dirname "$0")"
for shader in quad.vert quad.frag filter.vert filter.frag; do
    glslangValidator -V --target-env vulkan1.3 -Os "$shader" -o "$shader.spv"
done
