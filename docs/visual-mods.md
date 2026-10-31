# Visual Mods Catalog 🎨

MouseBlur includes **8 distinct visual mods**, each powered by custom GPU vector shaders, Catmull-Rom spline tessellation, or kinetic particle physics.

---

## 1. Optical Motion Blur (Default) 🌟
* **Internal Identifier**: `optical`
* **Category**: Authentic Physical Simulation
* **Description**: Simulates the shutter exposure of a physical high-speed camera. Computes continuous temporal motion blur with a crisp dark cursor border and luminous interior fill.
* **Algorithm**:
  * Evaluates cursor positions over the shutter duration (`tail_duration_ms`).
  * Renders a series of sub-pixel interpolated cursor geometries along the Catmull-Rom trajectory.
  * Applies an exponential alpha decay function:
    $$\alpha(t) = \alpha_{\text{base}} \cdot \left(1.0 - \frac{t}{\text{duration}}\right)^{1.4}$$
  * Outlines each sample with a 1.5px dark border for optimal contrast across light and dark desktop themes.

---

## 2. Directional Smear 🔀
* **Internal Identifier**: `directional`
* **Category**: Extruded Silhouette
* **Description**: Forms a solid, smooth tapered polygonal ribbon stretching behind the cursor.
* **Algorithm**:
  * Constructs a dynamic 2D mesh between trajectory points.
  * Extrudes orthogonal normal vectors $\vec{N}$ scaled by velocity:
    $$W(v) = W_{\text{head}} + (W_{\text{max}} - W_{\text{head}}) \cdot \min\left(1.0, \frac{v - v_{\text{min}}}{v_{\text{max}} - v_{\text{min}}}\right)$$
  * Fills the extruded quad strip with a Direct2D linear gradient brush.

---

## 3. Ghost Shutter Strobe ⏱️
* **Internal Identifier**: `ghost`
* **Category**: Temporal Afterimages
* **Description**: Generates discrete, fading pointer snapshots simulating a stroboscopic high-speed capture.
* **Algorithm**:
  * Samples historical positions at equidistant temporal intervals:
    $$\Delta t_{\text{sample}} = \frac{\text{duration}}{N_{\text{ghosts}}}$$
  * Draws distinct cursor shapes with linearly attenuated alpha.

---

## 4. Neon Bloom Laser ⚡
* **Internal Identifier**: `neon`
* **Category**: Luminous Glow
* **Description**: High-energy dual-pass laser beam.
* **Algorithm**:
  * **Pass 1 (Core)**: Renders a high-intensity narrow spline ribbon with near-100% opacity.
  * **Pass 2 (Ambient Bloom)**: Renders a wide, diffused outer ribbon with `glow_radius` expansion (up to 25px) and low alpha (35%) to create a soft atmospheric neon illumination.

---

## 5. Particle Sparks ✨
* **Internal Identifier**: `sparks`
* **Category**: Kinetic Physics Simulation
* **Description**: Emits luminous embers and sparkling particles along fast flicks.
* **Algorithm**:
  * Particle spawn rate scales with cursor flick speed.
  * Each particle maintains a 2D velocity vector $\vec{v}_p$, lifetime decay $L \in [0, 1]$, and random directional jitter:
    $$\vec{x}_{t+\Delta t} = \vec{x}_t + \vec{v}_p \cdot \Delta t, \quad \vec{v}_p \leftarrow \vec{v}_p \cdot 0.92$$
  * Rendered as glowing circular points with size and alpha decaying over time.

---

## 6. Chromatic Aberration (RGB Split) 🌈
* **Internal Identifier**: `chromatic`
* **Category**: Prismatic Dispersion
* **Description**: Splits the cursor into distinct Red, Green, and Blue channels offset orthogonally along the movement vector, simulating optical lens diffraction.
* **Algorithm**:
  * Calculates trajectory tangent $\vec{T}$ and normal $\vec{N}$.
  * **Red Channel**: Offset along $+\vec{N} \cdot d_{\text{split}}$.
  * **Green Channel**: Positioned along the center trajectory.
  * **Blue Channel**: Offset along $-\vec{N} \cdot d_{\text{split}}$.
  * Blended via additive per-pixel composition.

---

## 7. Action Speed Lines 💨
* **Internal Identifier**: `speedlines`
* **Category**: Manga / Cinematic Action
* **Description**: Spawns multiple parallel dashed speed-lines trailing behind the cursor during rapid movements.
* **Algorithm**:
  * Computes parallel trajectory lanes offset laterally from the cursor center.
  * Renders tapered line segments along these lanes whose visibility triggers only when velocity exceeds `min_velocity`.

---

## 8. Plasma Lightning Arc ⚡
* **Internal Identifier**: `plasma`
* **Category**: Stochastic Electric Discharge
* **Description**: High-voltage electric plasma arcs that snap and crackle between historical cursor waypoints.
* **Algorithm**:
  * Applies stochastic displacement (jitter) using pseudo-random normal distributions:
    $$\vec{P}'_i = \vec{P}_i + \vec{N}_i \cdot \text{jitter} \cdot \sin(\omega t + \phi_i)$$
  * Connects points with jagged multi-segment bolts surrounded by an outer electric aura.

---

## 🎨 Color Themes

MouseBlur supports 6 instant presets plus custom RGBA:
| Preset | Hex Code | Visual Character |
| :--- | :--- | :--- |
| **Pure White** | `#FFFFFF` | Minimalist, clean, natural optical exposure |
| **Cyber Neon Cyan** | `#00F0FF` | Vibrant sci-fi cyberpunk glow |
| **Electric Purple** | `#A855F7` | Deep energetic violet |
| **Sunset Orange** | `#F97316` | Warm solar ember flare |
| **Matrix Green** | `#22C55E` | Digital terminal green |
| **Rainbow Wave** | *Dynamic* | Continuous 360° hue-cycling spectrum ($1.2^\circ$ per frame) |
