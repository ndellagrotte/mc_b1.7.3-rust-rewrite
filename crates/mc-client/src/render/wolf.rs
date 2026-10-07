//! `RenderWolf` (RenderWolf.java): `RenderLiving` with `ModelWolf`; the
//! model's `ticks` argument is the tail angle (`EntityWolf.setTailRotation`),
//! and `ModelWolf.setLivingAnimations` sets the shaking wolf's
//! `glColor3f(brightness × shading)` ([`super::living`] passes it on). The
//! texture is `EntityWolf.getEntityTexture()` (tame, angry, plain). Its
//! `func_25006_b` (`preRenderCallback`) is empty (RenderWolf.java:16-17).
