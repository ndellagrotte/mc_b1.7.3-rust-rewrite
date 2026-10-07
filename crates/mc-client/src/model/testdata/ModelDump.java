// Project-owned test-fixture generator for crates/mc-client/src/model (WP-3.5a).
// Not compiled by cargo. It runs the vanilla Model* classes (the oracle build of
// PLAN_v4 §4.1, build/oracle) and prints, for every model the renderers build,
// each ModelRenderer's rotation point, angles, mirror flag, the 8 box corners
// and the 6 faces (corner indices in stored order and the texel rectangle the
// face's UVs were made from, checked bit for bit against TexturedQuad's
// formula), then the parts' angles and rotation points after a fixed list of
// pose calls on one shared instance per model (as the renderers share them).
//
// Regenerate (from the repository root, after tools/oracle/build.sh):
//   . tools/oracle/env.sh   # with ROOT set to the repository root
//   javac -cp build/oracle:$CP -d build/model-dump crates/mc-client/src/model/testdata/ModelDump.java
//   java $JAVA_MATH_FLAGS -cp build/model-dump:build/oracle:$CP ModelDump > crates/mc-client/src/model/testdata/java_models.txt

import java.lang.reflect.Field;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;
import net.minecraft.src.*;

public class ModelDump {
    static StringBuilder out = new StringBuilder();

    static Field field(Class<?> c, String name) throws Exception {
        for (Class<?> k = c; k != null; k = k.getSuperclass()) {
            try {
                Field f = k.getDeclaredField(name);
                f.setAccessible(true);
                return f;
            } catch (NoSuchFieldException e) {
            }
        }
        throw new NoSuchFieldException(name);
    }

    /** The parts in the Rust order: names, with "name[i]" for array elements. */
    static List<ModelRenderer> parts(Object m, String[] names) throws Exception {
        List<ModelRenderer> l = new ArrayList<>();
        for (String n : names) {
            int b = n.indexOf('[');
            if (b < 0) {
                l.add((ModelRenderer) field(m.getClass(), n).get(m));
            } else {
                Object[] a = (Object[]) field(m.getClass(), n.substring(0, b)).get(m);
                l.add((ModelRenderer) a[Integer.parseInt(n.substring(b + 1, n.length() - 1))]);
            }
        }
        return l;
    }

    static String f(float v) {
        return Float.toString(v);
    }

    static void dumpGeometry(String kind, Object m, String[] names) throws Exception {
        out.append("model ").append(kind).append('\n');
        List<ModelRenderer> ps = parts(m, names);
        Field fc = field(ModelRenderer.class, "corners");
        Field ff = field(ModelRenderer.class, "faces");
        Field ftx = field(ModelRenderer.class, "textureOffsetX");
        Field fty = field(ModelRenderer.class, "textureOffsetY");
        for (int i = 0; i < ps.size(); i++) {
            ModelRenderer p = ps.get(i);
            out.append("part ").append(names[i])
               .append(" tex ").append(ftx.getInt(p)).append(' ').append(fty.getInt(p))
               .append(" rp ").append(f(p.rotationPointX)).append(' ').append(f(p.rotationPointY)).append(' ').append(f(p.rotationPointZ))
               .append(" ang ").append(f(p.rotateAngleX)).append(' ').append(f(p.rotateAngleY)).append(' ').append(f(p.rotateAngleZ))
               .append(" mirror ").append(p.mirror ? 1 : 0).append('\n');
            PositionTextureVertex[] c = (PositionTextureVertex[]) fc.get(p);
            TexturedQuad[] q = (TexturedQuad[]) ff.get(p);
            out.append("corners");
            for (PositionTextureVertex v : c) {
                out.append(' ').append(f((float) v.vector3D.xCoord)).append(' ').append(f((float) v.vector3D.yCoord)).append(' ').append(f((float) v.vector3D.zCoord));
            }
            out.append('\n');
            for (TexturedQuad t : q) {
                int[] idx = new int[4];
                for (int k = 0; k < 4; k++) {
                    idx[k] = -1;
                    for (int j = 0; j < 8; j++) {
                        if (t.vertexPositions[k].vector3D == c[j].vector3D) {
                            idx[k] = j;
                        }
                    }
                    if (idx[k] < 0) throw new IllegalStateException("vertex not a corner");
                }
                // Stored order before flipFace: [u2,v1] [u1,v1] [u1,v2] [u2,v2].
                PositionTextureVertex[] o = t.vertexPositions;
                if (p.mirror) {
                    o = new PositionTextureVertex[]{o[3], o[2], o[1], o[0]};
                }
                float i6 = 0.0015625F;
                float i7 = 0.003125F;
                int u1 = Math.round((o[1].texturePositionX - i6) * 64.0F);
                int v1 = Math.round((o[1].texturePositionY - i7) * 32.0F);
                int u2 = Math.round((o[0].texturePositionX + i6) * 64.0F);
                int v2 = Math.round((o[2].texturePositionY + i7) * 32.0F);
                float[][] want = {
                    {(float) u2 / 64.0F - i6, (float) v1 / 32.0F + i7},
                    {(float) u1 / 64.0F + i6, (float) v1 / 32.0F + i7},
                    {(float) u1 / 64.0F + i6, (float) v2 / 32.0F - i7},
                    {(float) u2 / 64.0F - i6, (float) v2 / 32.0F - i7},
                };
                for (int k = 0; k < 4; k++) {
                    if (Float.floatToIntBits(o[k].texturePositionX) != Float.floatToIntBits(want[k][0])
                            || Float.floatToIntBits(o[k].texturePositionY) != Float.floatToIntBits(want[k][1])) {
                        throw new IllegalStateException("uv not from a texel rect: " + kind + " " + names[i]);
                    }
                }
                out.append("face ").append(idx[0]).append(' ').append(idx[1]).append(' ').append(idx[2]).append(' ').append(idx[3])
                   .append(' ').append(u1).append(' ').append(v1).append(' ').append(u2).append(' ').append(v2).append('\n');
            }
        }
    }

    static void dumpPose(String kind, String name, Object m, String[] names) throws Exception {
        out.append("pose ").append(kind).append(' ').append(name).append('\n');
        for (ModelRenderer p : parts(m, names)) {
            out.append(f(p.rotationPointX)).append(' ').append(f(p.rotationPointY)).append(' ').append(f(p.rotationPointZ))
               .append(' ').append(f(p.rotateAngleX)).append(' ').append(f(p.rotateAngleY)).append(' ').append(f(p.rotateAngleZ)).append('\n');
        }
    }

    static final float[][] ARGS = {
        {0.0F, 0.0F, 0.0F, 0.0F, 0.0F, 0.0625F},
        {1.3F, 0.7F, 25.5F, 30.0F, -20.0F, 0.0625F},
        {7.77F, 1.0F, 1234.25F, -75.5F, 45.0F, 0.0625F},
    };

    static void poses(String kind, ModelBase m, String[] names) throws Exception {
        for (int i = 0; i < ARGS.length; i++) {
            float[] a = ARGS[i];
            m.setRotationAngles(a[0], a[1], a[2], a[3], a[4], a[5]);
            dumpPose(kind, "args" + i, m, names);
        }
    }

    static final String[] BIPED = {"bipedHead", "bipedHeadwear", "bipedBody", "bipedRightArm", "bipedLeftArm", "bipedRightLeg", "bipedLeftLeg", "bipedEars", "bipedCloak"};
    static final String[] QUAD = {"head", "body", "leg1", "leg2", "leg3", "leg4"};
    static final String[] COW = {"head", "body", "leg1", "leg2", "leg3", "leg4", "udders", "horn1", "horn2"};
    static final String[] CHICKEN = {"head", "body", "rightLeg", "leftLeg", "rightWing", "leftWing", "bill", "chin"};
    static final String[] CREEPER = {"head", "unusedCreeperHeadwear", "body", "leg1", "leg2", "leg3", "leg4"};
    static final String[] SPIDER = {"spiderHead", "spiderNeck", "spiderBody", "spiderLeg1", "spiderLeg2", "spiderLeg3", "spiderLeg4", "spiderLeg5", "spiderLeg6", "spiderLeg7", "spiderLeg8"};
    static final String[] WOLF = {"wolfHeadMain", "wolfBody", "wolfLeg1", "wolfLeg2", "wolfLeg3", "wolfLeg4", "wolfRightEar", "wolfLeftEar", "wolfSnout", "wolfTail", "wolfMane"};
    static final String[] SLIME = {"slimeBodies", "slimeRightEye", "slimeLeftEye", "slimeMouth"};
    static final String[] SLIME_OUTER = {"slimeBodies"};
    static final String[] MINECART = {"sideModels[0]", "sideModels[1]", "sideModels[2]", "sideModels[3]", "sideModels[4]", "sideModels[5]"};
    static final String[] BOAT = {"boatSides[0]", "boatSides[1]", "boatSides[2]", "boatSides[3]", "boatSides[4]"};

    static String[] indexed(String a, int n, String first) {
        String[] r = new String[n + 1];
        r[0] = first;
        for (int i = 0; i < n; i++) r[i + 1] = a + "[" + i + "]";
        return r;
    }

    static void biped(String kind, ModelBiped m) throws Exception {
        dumpGeometry(kind, m, BIPED);
        poses(kind, m, BIPED);
        float[] a = ARGS[1];
        m.onGround = 0.3F;
        m.isRiding = true;
        m.heldItemRight = true;
        m.setRotationAngles(a[0], a[1], a[2], a[3], a[4], a[5]);
        dumpPose(kind, "swing_riding_held", m, BIPED);
        a = ARGS[2];
        m.onGround = 0.85F;
        m.isRiding = false;
        m.heldItemRight = false;
        m.heldItemLeft = true;
        m.isSneak = true;
        m.setRotationAngles(a[0], a[1], a[2], a[3], a[4], a[5]);
        dumpPose(kind, "swing_sneak_left", m, BIPED);
        m.onGround = 0.0F;
        m.heldItemLeft = false;
        m.isSneak = false;
        m.setRotationAngles(a[0], a[1], a[2], a[3], a[4], a[5]);
        dumpPose(kind, "reset", m, BIPED);
        // renderEars' side effects (RenderPlayer), without the GL call.
        ModelRenderer ears = m.bipedEars;
        ears.rotateAngleY = m.bipedHead.rotateAngleY;
        ears.rotateAngleX = m.bipedHead.rotateAngleX;
        ears.rotationPointX = 0.0F;
        ears.rotationPointY = 0.0F;
        dumpPose(kind, "ears", m, BIPED);
    }

    public static void main(String[] args) throws Exception {
        biped("Biped", new ModelBiped());
        biped("BipedArmor", new ModelBiped(0.5F));
        biped("BipedArmorChestplate", new ModelBiped(1.0F));
        biped("Zombie", new ModelZombie());
        biped("Skeleton", new ModelSkeleton());

        Object[][] simple = {
            {"Pig", new ModelPig(), QUAD},
            {"PigSaddle", new ModelPig(0.5F), QUAD},
            {"Cow", new ModelCow(), COW},
            {"Sheep", new ModelSheep2(), QUAD},
            {"SheepFur", new ModelSheep1(), QUAD},
            {"Chicken", new ModelChicken(), CHICKEN},
            {"Creeper", new ModelCreeper(), CREEPER},
            {"CreeperArmor", new ModelCreeper(2.0F), CREEPER},
            {"Spider", new ModelSpider(), SPIDER},
            {"Slime", new ModelSlime(16), SLIME},
            {"SlimeOuter", new ModelSlime(0), SLIME_OUTER},
            {"Ghast", new ModelGhast(), indexed("tentacles", 9, "body")},
            {"Squid", new ModelSquid(), indexed("squidTentacles", 8, "squidBody")},
            {"Minecart", new ModelMinecart(), MINECART},
            {"Boat", new ModelBoat(), BOAT},
        };
        for (Object[] s : simple) {
            dumpGeometry((String) s[0], s[1], (String[]) s[2]);
            poses((String) s[0], (ModelBase) s[1], (String[]) s[2]);
        }

        // The wolf: setLivingAnimations then setRotationAngles, as
        // RenderLiving.doRenderLiving calls them (RenderLiving.java:52-53).
        ModelWolf w = new ModelWolf();
        dumpGeometry("Wolf", w, WOLF);
        EntityWolf e = new EntityWolf(null);
        Field shaking = field(EntityWolf.class, "timeWolfIsShaking");
        Field prevShaking = field(EntityWolf.class, "prevTimeWolfIsShaking");
        Field course = field(EntityWolf.class, "headRotationCourse");
        Field courseOld = field(EntityWolf.class, "headRotationCourseOld");
        // case: plain
        w.setLivingAnimations(e, 1.3F, 0.7F, 0.25F);
        w.setRotationAngles(1.3F, 0.7F, 0.6F, 30.0F, -20.0F, 0.0625F);
        dumpPose("Wolf", "plain", w, WOLF);
        // case: sitting, interested, mid-shake (getWolfShaking false: no glColor)
        e.setWolfSitting(true);
        course.setFloat(e, 0.8F);
        courseOld.setFloat(e, 0.5F);
        shaking.setFloat(e, 0.9F);
        prevShaking.setFloat(e, 0.75F);
        w.setLivingAnimations(e, 7.77F, 1.0F, 0.6F);
        w.setRotationAngles(7.77F, 1.0F, 1.2F, -75.5F, 45.0F, 0.0625F);
        dumpPose("Wolf", "sitting_shake", w, WOLF);
        // case: angry, standing again
        e.setWolfSitting(false);
        e.setWolfAngry(true);
        w.setLivingAnimations(e, 2.5F, 0.4F, 0.9F);
        w.setRotationAngles(2.5F, 0.4F, 1.5707964F, 10.0F, 5.0F, 0.0625F);
        dumpPose("Wolf", "angry", w, WOLF);

        Random r = new Random(1660L);
        out.append("ghast_lengths");
        for (int i = 0; i < 9; i++) out.append(' ').append(r.nextInt(7) + 8);
        out.append('\n');
        System.out.print(out);
    }
}
