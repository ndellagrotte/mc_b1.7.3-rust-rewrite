//! `RenderLiving` (RenderLiving.java): `doRenderLiving` transcribed once,
//! with the methods its subclasses override (`preRenderCallback`,
//! `rotateCorpse`, `shouldRenderPass`, `getColorMultiplier`, …) dispatched
//! by [`LivingClass`] to their files ([`super::player`], [`super::creeper`],
//! …). A renderer owns its models as the Java does (`mainModel`, and the
//! models `setRenderPassModel` can point at), so pose state carries between
//! entities of one class as it does in vanilla.
//!
//! The draw, per entity: `glDisable(GL_CULL_FACE)`; the swing progress and
//! riding flag into the main model (and into the current pass model);
//! translate to the entity, rotate `180 - renderYawOffset` (and the death
//! tilt), `glScalef(-1, -1, 1)`, the subclass scale, translate `(0, -1.5 -
//! 1/128, 0)`; bind the entity's texture; the main model; up to four pass
//! models (`shouldRenderPass`); the held items; then, while hurt, dying or
//! with a colour multiplier, the same models again untextured, blended and
//! `GL_EQUAL` to the depth just written: red `(brightness, 0, 0, 0.4)`
//! and/or the multiplier's colour. With F3 on, the entity id over it
//! (`passSpecialRender`).

use mc_core::math::sqrt_float;
use mc_sim::entity::{Entity, EntityKindData as K, dispatch, living as eliving};

use super::base::RenderBase;
use super::gl::glx;
use super::{Ctx, RendererId};
use crate::gui::batch::gl;
use crate::model::wolf::WolfAnimInputs;
use crate::model::{LivingInputs, Model, ModelKind};

/// Which `RenderLiving` subclass (the overrides).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LivingClass {
    /// `RenderLiving` itself.
    Living,
    /// `RenderPlayer`.
    Player,
    /// `RenderBiped`.
    Biped,
    /// `RenderCreeper`.
    Creeper,
    /// `RenderSpider`.
    Spider,
    /// `RenderSheep`.
    Sheep,
    /// `RenderWolf`.
    Wolf,
    /// `RenderPig`.
    Pig,
    /// `RenderCow` (no override).
    Cow,
    /// `RenderChicken`.
    Chicken,
    /// `RenderSquid`.
    Squid,
    /// `RenderSlime`.
    Slime,
    /// `RenderGhast`.
    Ghast,
    /// `RenderGiantZombie`.
    GiantZombie,
}

/// A `RenderLiving` instance (module docs).
#[derive(Clone, Debug)]
pub struct RenderLiving {
    /// The subclass.
    pub class: LivingClass,
    /// `shadowSize`, `field_194_c`.
    pub base: RenderBase,
    /// `mainModel`.
    pub main_model: Model,
    /// The other models the renderer holds (`renderPassModel` points at
    /// one of them): the player's `modelArmorChestplate` and `modelArmor`,
    /// the pig's saddle, the sheep's wool, the spider's eye model, the
    /// creeper's `field_27008_a`, the slime's `scaleAmount`.
    pub models: Vec<Model>,
    /// `renderPassModel`, as an index into [`Self::models`].
    pub render_pass_model: Option<usize>,
    /// `RenderGiantZombie.scale`.
    pub scale: f32,
}

impl RenderLiving {
    /// The instance `RenderManager`'s constructor makes for `id`.
    // src: RenderManager.java:27-41, RenderSpider.java:6-9, RenderCreeper.java:6-10,
    // RenderPlayer.java:12-17, RenderGhast.java:6-8, RenderGiantZombie.java:8-11,
    // RenderSlime.java:8-11, RenderPig.java:4-7, RenderSheep.java:6-9
    pub fn new(id: RendererId) -> Self {
        use ModelKind as M;
        let (class, main, models, pass, shadow, scale): (
            LivingClass,
            ModelKind,
            Vec<ModelKind>,
            Option<usize>,
            f32,
            f32,
        ) = match id {
            RendererId::Spider => (
                LivingClass::Spider,
                M::Spider,
                vec![M::Spider],
                Some(0),
                1.0,
                1.0,
            ),
            RendererId::Pig => (
                LivingClass::Pig,
                M::Pig,
                vec![M::PigSaddle],
                Some(0),
                0.7,
                1.0,
            ),
            RendererId::Sheep => (
                LivingClass::Sheep,
                M::Sheep,
                vec![M::SheepFur],
                Some(0),
                0.7,
                1.0,
            ),
            RendererId::Cow => (LivingClass::Cow, M::Cow, vec![], None, 0.7, 1.0),
            RendererId::Wolf => (LivingClass::Wolf, M::Wolf, vec![], None, 0.5, 1.0),
            RendererId::Chicken => (LivingClass::Chicken, M::Chicken, vec![], None, 0.3, 1.0),
            RendererId::Creeper => (
                LivingClass::Creeper,
                M::Creeper,
                vec![M::CreeperArmor],
                None,
                0.5,
                1.0,
            ),
            RendererId::Skeleton => (LivingClass::Biped, M::Skeleton, vec![], None, 0.5, 1.0),
            RendererId::Zombie => (LivingClass::Biped, M::Zombie, vec![], None, 0.5, 1.0),
            RendererId::Slime => (
                LivingClass::Slime,
                M::Slime,
                vec![M::SlimeOuter],
                None,
                0.25,
                1.0,
            ),
            RendererId::Player => (
                LivingClass::Player,
                M::Biped,
                vec![M::BipedArmorChestplate, M::BipedArmor],
                None,
                0.5,
                1.0,
            ),
            // super(model, 0.5F * 6.0F)
            RendererId::GiantZombie => (
                LivingClass::GiantZombie,
                M::Zombie,
                vec![],
                None,
                0.5 * 6.0,
                6.0,
            ),
            RendererId::Ghast => (LivingClass::Ghast, M::Ghast, vec![], None, 0.5, 1.0),
            RendererId::Squid => (LivingClass::Squid, M::Squid, vec![], None, 0.7, 1.0),
            _ => (LivingClass::Living, M::Biped, vec![], None, 0.5, 1.0),
        };
        RenderLiving {
            class,
            base: RenderBase::with_shadow(shadow),
            main_model: Model::new(main),
            models: models.into_iter().map(Model::new).collect(),
            render_pass_model: pass,
            scale,
        }
    }

    /// `setRenderPassModel(model)`.
    // src: RenderLiving.java:15-17
    pub fn set_render_pass_model(&mut self, i: usize) {
        self.render_pass_model = Some(i);
    }

    /// `doRender` (the bridge to `doRenderLiving`, or the subclass's
    /// wrapper: `RenderPlayer.renderPlayer`).
    // src: RenderLiving.java:217-219, RenderPlayer.java:516-518
    #[allow(clippy::too_many_arguments)]
    pub fn do_render(
        &mut self,
        cx: &mut Ctx,
        e: &Entity,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        partial: f32,
    ) {
        if e.kind.living().is_none() {
            return;
        }
        match self.class {
            LivingClass::Player => super::player::render_player(self, cx, e, x, y, z, yaw, partial),
            _ => self.do_render_living(cx, e, x, y, z, yaw, partial),
        }
    }

    /// `doRenderLiving(entity, x, y, z, yaw, partial)` (module docs).
    // src: RenderLiving.java:19-114
    #[allow(clippy::too_many_arguments)]
    pub fn do_render_living(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        var2: f64,
        var4: f64,
        var6: f64,
        _var8: f32,
        var9: f32,
    ) {
        cx.gl.push_matrix();
        cx.gl.disable(gl::CULL_FACE);
        self.main_model.on_ground = self.func_167_c(var1, var9);
        if let Some(i) = self.render_pass_model {
            self.models[i].on_ground = self.main_model.on_ground;
        }
        self.main_model.is_riding = var1.base.is_riding();
        if let Some(i) = self.render_pass_model {
            self.models[i].is_riding = self.main_model.is_riding;
        }
        // try { … } catch (Exception) { printStackTrace }: a pass model
        // missing where the Java would throw ends the block early.
        self.do_render_living_body(cx, var1, var2, var4, var6, var9);
        cx.gl.enable(gl::CULL_FACE);
        cx.gl.pop_matrix();
        self.pass_special_render(cx, var1, var2, var4, var6);
    }

    /// The `try` block of `doRenderLiving`.
    // src: RenderLiving.java:32-106
    fn do_render_living_body(
        &mut self,
        cx: &mut Ctx,
        var1: &Entity,
        var2: f64,
        var4: f64,
        var6: f64,
        var9: f32,
    ) -> Option<()> {
        let l = var1.kind.living()?;
        let b = &var1.base;
        let var10 =
            l.prev_render_yaw_offset + (l.render_yaw_offset - l.prev_render_yaw_offset) * var9;
        let var11 = b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var9;
        let var12 = b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var9;
        self.func_22012_b(cx, var1, var2, var4, var6);
        let var13 = self.func_170_d(var1, var9);
        self.rotate_corpse(cx, var1, var13, var10, var9);
        let var14 = 0.0625f32;
        cx.gl.enable(gl::RESCALE_NORMAL);
        cx.gl.scalef(-1.0, -1.0, 1.0);
        self.pre_render_callback(cx, var1, var9);
        cx.gl.translatef(0.0, -24.0 * var14 - 0.0078125, 0.0);
        let mut var15 = l.prev_leg_yaw + (l.leg_yaw - l.prev_leg_yaw) * var9;
        let var16 = l.leg_swing - l.leg_yaw * (1.0 - var9);
        if var15 > 1.0 {
            var15 = 1.0;
        }
        // loadDownloadableImageTexture(skinUrl, getEntityTexture()): skins are
        // downloads (D2), so the fallback texture is bound, as when the
        // download failed (RenderEngine.getTextureForDownloadableImage).
        // src: RenderLiving.java:50
        if let Some(t) = dispatch::get_entity_texture(var1, cx.world) {
            cx.gl.bind_texture(t);
        }
        cx.gl.enable(gl::ALPHA_TEST);
        // mainModel.setLivingAnimations (only ModelWolf's does something)
        let inputs = match var1.kind {
            K::Wolf(..) => {
                let br = cx.brightness(var1, var9);
                LivingInputs::Wolf(WolfAnimInputs::from_entity(var1, var9, br))
            }
            _ => LivingInputs::None,
        };
        if let Some([r, g, bl]) = self
            .main_model
            .set_living_animations(&inputs, var16, var15, var9)
        {
            cx.gl.color3f(r, g, bl);
        }
        let args = (var16, var15, var13, var11 - var10, var12, var14);
        self.render_main(cx, args);
        for var17 in 0..4 {
            if self.should_render_pass(cx, var1, var17, var9) {
                self.render_pass(cx, args)?;
                cx.gl.disable(gl::BLEND);
                cx.gl.enable(gl::ALPHA_TEST);
            }
        }
        self.render_equipped_items(cx, var1, var9);
        let var25 = cx.brightness(var1, var9);
        let var18 = self.get_color_multiplier(var1, var25, var9);
        if (var18 >> 24 & 255) > 0 || l.hurt_time > 0 || l.death_time > 0 {
            cx.gl.disable(gl::TEXTURE_2D);
            cx.gl.disable(gl::ALPHA_TEST);
            cx.gl.enable(gl::BLEND);
            cx.gl.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            cx.gl.depth_func(glx::EQUAL);
            if l.hurt_time > 0 || l.death_time > 0 {
                cx.gl.color4f(var25, 0.0, 0.0, 0.4);
                self.render_main(cx, args);
                for var19 in 0..4 {
                    if self.func_27005_b(cx, var1, var19, var9) {
                        cx.gl.color4f(var25, 0.0, 0.0, 0.4);
                        self.render_pass(cx, args)?;
                    }
                }
            }
            if (var18 >> 24 & 255) > 0 {
                let var26 = (var18 >> 16 & 255) as f32 / 255.0;
                let var20 = (var18 >> 8 & 255) as f32 / 255.0;
                let var21 = (var18 & 255) as f32 / 255.0;
                let var22 = (var18 >> 24 & 255) as f32 / 255.0;
                cx.gl.color4f(var26, var20, var21, var22);
                self.render_main(cx, args);
                for var23 in 0..4 {
                    if self.func_27005_b(cx, var1, var23, var9) {
                        cx.gl.color4f(var26, var20, var21, var22);
                        self.render_pass(cx, args)?;
                    }
                }
            }
            cx.gl.depth_func(glx::LEQUAL);
            cx.gl.disable(gl::BLEND);
            cx.gl.enable(gl::ALPHA_TEST);
            cx.gl.enable(gl::TEXTURE_2D);
        }
        cx.gl.disable(gl::RESCALE_NORMAL);
        Some(())
    }

    /// `mainModel.render(…)`.
    fn render_main(&mut self, cx: &mut Ctx, (a, b, c, d, e, f): (f32, f32, f32, f32, f32, f32)) {
        let draws = self.main_model.render(a, b, c, d, e, f);
        cx.gl.model(self.main_model.kind, draws);
    }

    /// `renderPassModel.render(…)`; `None` where the Java's null
    /// `renderPassModel` would throw.
    fn render_pass(
        &mut self,
        cx: &mut Ctx,
        (a, b, c, d, e, f): (f32, f32, f32, f32, f32, f32),
    ) -> Option<()> {
        let m = &mut self.models[self.render_pass_model?];
        let draws = m.render(a, b, c, d, e, f);
        cx.gl.model(m.kind, draws);
        Some(())
    }

    // ---- the overridable methods ----------------------------------------------------

    /// `func_167_c(entity, partial)`: the swing progress.
    // src: RenderLiving.java:134-136
    fn func_167_c(&self, var1: &Entity, var2: f32) -> f32 {
        eliving::get_swing_progress(var1, var2)
    }

    /// `func_22012_b(entity, x, y, z)`: the translation (the sleeping
    /// player's offset).
    // src: RenderLiving.java:116-118, RenderPlayer.java:232-239
    fn func_22012_b(&self, cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
        match self.class {
            LivingClass::Player => super::player::func_22016_b(cx, var1, var2, var4, var6),
            _ => cx.gl.translatef(var2 as f32, var4 as f32, var6 as f32),
        }
    }

    /// `func_170_d(entity, partial)`: the model's `ticks` argument.
    // src: RenderLiving.java:138-140, RenderWolf.java:12-14, RenderChicken.java:12-16,
    // RenderSquid.java:27-30
    fn func_170_d(&self, var1: &Entity, var2: f32) -> f32 {
        match self.class {
            LivingClass::Wolf => mc_sim::entity::wolf::set_tail_rotation(var1),
            LivingClass::Chicken => super::chicken::get_wing_rotation(var1, var2),
            LivingClass::Squid => super::squid::func_21006_b(var1, var2),
            _ => var1.base.ticks_existed as f32 + var2,
        }
    }

    /// `rotateCorpse(entity, ticks, bodyYaw, partial)`.
    // src: RenderLiving.java:120-132
    fn rotate_corpse(&self, cx: &mut Ctx, var1: &Entity, var2: f32, var3: f32, var4: f32) {
        match self.class {
            LivingClass::Player => super::player::func_22017_a(self, cx, var1, var2, var3, var4),
            LivingClass::Squid => super::squid::func_21007_a(cx, var1, var2, var3, var4),
            _ => self.rotate_corpse_base(cx, var1, var3, var4),
        }
    }

    /// `RenderLiving.rotateCorpse`: the body yaw and the death tilt
    /// ([`death_rotation`]).
    // src: RenderLiving.java:120-132
    pub fn rotate_corpse_base(&self, cx: &mut Ctx, var1: &Entity, var3: f32, var4: f32) {
        cx.gl.rotatef(180.0 - var3, 0.0, 1.0, 0.0);
        let death_time = var1.kind.living().map_or(0, |l| l.death_time);
        if death_time > 0 {
            let var5 = death_rotation(death_time, var4);
            cx.gl
                .rotatef(var5 * self.get_death_max_rotation(), 0.0, 0.0, 1.0);
        }
    }

    /// `getDeathMaxRotation(entity)`: 90, the spider's 180.
    // src: RenderLiving.java:153-155, RenderSpider.java:11-13
    pub fn get_death_max_rotation(&self) -> f32 {
        match self.class {
            LivingClass::Spider => 180.0,
            _ => 90.0,
        }
    }

    /// `preRenderCallback(entity, partial)`.
    // src: RenderLiving.java:161-162
    fn pre_render_callback(&self, cx: &mut Ctx, var1: &Entity, var2: f32) {
        match self.class {
            LivingClass::Player => super::player::func_186_b(cx),
            LivingClass::Creeper => super::creeper::update_creeper_scale(cx, var1, var2),
            LivingClass::Slime => super::slime::scale_slime(cx, var1, var2),
            LivingClass::Ghast => super::ghast::func_4014_a(cx, var1, var2),
            LivingClass::GiantZombie => super::giant_zombie::pre_render_scale(cx, self.scale),
            _ => {}
        }
    }

    /// `shouldRenderPass(entity, pass, partial)`.
    // src: RenderLiving.java:149-151
    fn should_render_pass(&mut self, cx: &mut Ctx, var1: &Entity, var2: i32, var3: f32) -> bool {
        match self.class {
            LivingClass::Player => super::player::set_armor_model(self, cx, var1, var2),
            LivingClass::Creeper => super::creeper::func_27006_a(self, cx, var1, var2, var3),
            LivingClass::Spider => super::spider::set_spider_eye_brightness(cx, var1, var2),
            LivingClass::Sheep => super::sheep::set_wool_color_and_render(cx, var1, var2, var3),
            LivingClass::Pig => super::pig::render_saddled_pig(cx, var1, var2),
            LivingClass::Slime => super::slime::render_slime_pass_model(self, cx, var2),
            _ => false,
        }
    }

    /// `func_27005_b(entity, pass, partial)`: the hurt overlay's pass
    /// check (`shouldRenderPass` but for the creeper).
    // src: RenderLiving.java:145-147, RenderCreeper.java:83-85
    fn func_27005_b(&mut self, cx: &mut Ctx, var1: &Entity, var2: i32, var3: f32) -> bool {
        match self.class {
            LivingClass::Creeper => false,
            _ => self.should_render_pass(cx, var1, var2, var3),
        }
    }

    /// `getColorMultiplier(entity, brightness, partial)` (ARGB).
    // src: RenderLiving.java:157-159
    fn get_color_multiplier(&self, var1: &Entity, _var2: f32, var3: f32) -> i32 {
        match self.class {
            LivingClass::Creeper => super::creeper::update_creeper_color_multiplier(var1, var3),
            _ => 0,
        }
    }

    /// `renderEquippedItems(entity, partial)`.
    // src: RenderLiving.java:142-143
    fn render_equipped_items(&mut self, cx: &mut Ctx, var1: &Entity, var2: f32) {
        match self.class {
            LivingClass::Player => super::player::render_specials(self, cx, var1, var2),
            LivingClass::Biped => super::biped::render_equipped_items(self, cx, var1),
            _ => {}
        }
    }

    /// `passSpecialRender(entity, x, y, z)`: with F3, the entity id above
    /// it; the player's `renderName` instead.
    // src: RenderLiving.java:164-169, RenderPlayer.java:474-476
    fn pass_special_render(&self, cx: &mut Ctx, var1: &Entity, var2: f64, var4: f64, var6: f64) {
        match self.class {
            LivingClass::Player => super::player::render_name(cx, var1, var2, var4, var6),
            _ => {
                if cx.info.debug_info {
                    let s = var1.base.entity_id.to_string();
                    render_living_label(cx, var1, &s, var2, var4, var6, 64);
                }
            }
        }
    }
}

/// The death tilt's fraction: `sqrt((deathTime + partial - 1) / 20 × 1.6)`,
/// at most 1.
// src: RenderLiving.java:123-127
pub fn death_rotation(death_time: i32, var4: f32) -> f32 {
    let mut var5 = (death_time as f32 + var4 - 1.0) / 20.0 * 1.6;
    var5 = sqrt_float(var5);
    if var5 > 1.0 {
        var5 = 1.0;
    }
    var5
}

/// `renderLivingLabel(entity, text, x, y, z, maxDistance)`: the text 2.3
/// above the entity facing the camera, `0.016666668 × 1.6` per pixel, on a
/// dark quad of alpha 0.25; drawn once through everything (no depth test,
/// colour `0x20FFFFFF`) and once depth-tested in white. Lighting off,
/// blended.
// src: RenderLiving.java:171-213
#[allow(clippy::too_many_arguments)]
pub fn render_living_label(
    cx: &mut Ctx,
    var1: &Entity,
    var2: &str,
    var3: f64,
    var5: f64,
    var7: f64,
    var9: i32,
) {
    let Some((_, lp)) = cx.info.living_player else {
        return;
    };
    let b = &var1.base;
    // getDistanceToEntity(livingPlayer) (Entity.java:653-658)
    let (dx, dy, dz) = (
        (b.pos_x - lp.x) as f32,
        (b.pos_y - lp.y) as f32,
        (b.pos_z - lp.z) as f32,
    );
    let var10 = sqrt_float(dx * dx + dy * dy + dz * dz);
    if var10 > var9 as f32 {
        return;
    }
    let var12 = 1.6f32;
    let var13 = 0.016666668 * var12;
    let (vy, vx) = (cx.info.player_view_y, cx.info.player_view_x);
    let font = cx.font;
    let g = &mut *cx.gl;
    g.push_matrix();
    g.translatef(var3 as f32 + 0.0, var5 as f32 + 2.3, var7 as f32);
    g.normal3f(0.0, 1.0, 0.0);
    g.rotatef(-vy, 0.0, 1.0, 0.0);
    g.rotatef(vx, 1.0, 0.0, 0.0);
    g.scalef(-var13, -var13, var13);
    g.disable(gl::LIGHTING);
    g.depth_mask(false);
    g.disable(gl::DEPTH_TEST);
    g.enable(gl::BLEND);
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    let mut var15: i32 = 0;
    if var2 == "deadmau5" {
        var15 = -10;
    }
    g.disable(gl::TEXTURE_2D);
    g.start_drawing_quads();
    let var16 = font.get_string_width(var2) / 2;
    g.set_color_rgba_f(0.0, 0.0, 0.0, 0.25);
    g.add_vertex((-var16 - 1) as f64, (-1 + var15) as f64, 0.0);
    g.add_vertex((-var16 - 1) as f64, (8 + var15) as f64, 0.0);
    g.add_vertex((var16 + 1) as f64, (8 + var15) as f64, 0.0);
    g.add_vertex((var16 + 1) as f64, (-1 + var15) as f64, 0.0);
    g.draw();
    g.enable(gl::TEXTURE_2D);
    super::text3d::draw_string(
        g,
        font,
        var2,
        -font.get_string_width(var2) / 2,
        var15,
        553648127,
    );
    g.enable(gl::DEPTH_TEST);
    g.depth_mask(true);
    super::text3d::draw_string(g, font, var2, -font.get_string_width(var2) / 2, var15, -1);
    g.enable(gl::LIGHTING);
    g.disable(gl::BLEND);
    g.color4f(1.0, 1.0, 1.0, 1.0);
    g.pop_matrix();
}
