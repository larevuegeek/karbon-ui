//! Suite minimale du renderer Direct2D / DirectWrite : scène → pixels sur WARP (DPI 1x et
//! mis à l'échelle), clips et alpha, chaîne d'échange, mesure du texte ; le test GPU
//! rejoue les mêmes vérifications sur le matériel.
mod effects;
mod gpu;
mod gradients;
mod paint;
mod shapes;
mod svg;
mod swapchain;
mod text;
