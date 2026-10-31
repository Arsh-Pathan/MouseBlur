use std::mem::ManuallyDrop;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::DirectComposition::*;
use windows::Win32::Graphics::Direct2D::*;
use windows::Win32::Graphics::Direct2D::Common::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::Graphics::Dxgi::Common::*;
use windows_numerics::Vector2;

use crate::config::{BlurMode, Config};
use crate::trail::MouseTracker;

pub struct Renderer {
    _d3d_device: ID3D11Device,
    _d3d_context: ID3D11DeviceContext,
    swap_chain: IDXGISwapChain1,
    _dcomp_device: IDCompositionDevice,
    _dcomp_target: IDCompositionTarget,
    _dcomp_visual: IDCompositionVisual,
    d2d_factory: ID2D1Factory1,
    d2d_context: ID2D1DeviceContext,
    target_bitmap: Option<ID2D1Bitmap1>,
    brush: ID2D1SolidColorBrush,
    glow_brush: ID2D1SolidColorBrush,
    outline_brush: ID2D1SolidColorBrush,
    red_brush: ID2D1SolidColorBrush,
    green_brush: ID2D1SolidColorBrush,
    blue_brush: ID2D1SolidColorBrush,
    dynamic_brush: ID2D1SolidColorBrush,
    arrow_geometry: ID2D1PathGeometry,
    width: u32,
    height: u32,
    rainbow_phase: f32,
}

impl Renderer {
    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self> {
        let width = width.max(1);
        let height = height.max(1);

        unsafe {
            // 1. Create D3D11 Device with BGRA Support
            let feature_levels = [
                D3D_FEATURE_LEVEL_11_1,
                D3D_FEATURE_LEVEL_11_0,
                D3D_FEATURE_LEVEL_10_1,
            ];
            let mut d3d_device_opt: Option<ID3D11Device> = None;
            let mut d3d_context_opt: Option<ID3D11DeviceContext> = None;
            let mut actual_fl = D3D_FEATURE_LEVEL_11_0;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut d3d_device_opt),
                Some(&mut actual_fl),
                Some(&mut d3d_context_opt),
            )?;

            let d3d_device = d3d_device_opt.ok_or_else(|| Error::from(E_FAIL))?;
            let d3d_context = d3d_context_opt.ok_or_else(|| Error::from(E_FAIL))?;

            // 2. Query DXGI Factory
            let dxgi_device: IDXGIDevice = d3d_device.cast()?;
            let dxgi_adapter = dxgi_device.GetAdapter()?;
            let dxgi_factory: IDXGIFactory2 = dxgi_adapter.GetParent()?;

            // 3. Create Swap Chain for DirectComposition
            let sc_desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                Stereo: false.into(),
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                Scaling: DXGI_SCALING_STRETCH,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
                Flags: 0,
            };

            let swap_chain = dxgi_factory.CreateSwapChainForComposition(&dxgi_device, &sc_desc, None)?;

            // 4. Create DirectComposition Device and bind to Window
            let dcomp_device: IDCompositionDevice = DCompositionCreateDevice(&dxgi_device)?;
            let dcomp_target = dcomp_device.CreateTargetForHwnd(hwnd, true)?;
            let dcomp_visual = dcomp_device.CreateVisual()?;
            dcomp_visual.SetContent(&swap_chain)?;
            dcomp_target.SetRoot(&dcomp_visual)?;
            dcomp_device.Commit()?;

            // 5. Create Direct2D Factory and Device Context
            let d2d_factory: ID2D1Factory1 = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let d2d_device = d2d_factory.CreateDevice(&dxgi_device)?;
            let d2d_context = d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

            // 6. Bind swap chain back-buffer to Direct2D
            let surface: IDXGISurface = swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: ManuallyDrop::new(None),
            };

            let target_bitmap = d2d_context.CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))?;
            d2d_context.SetTarget(&target_bitmap);

            // 7. Create solid brushes
            let brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.94,
                    b: 1.0,
                    a: 0.85,
                },
                None,
            )?;

            let glow_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.94,
                    b: 1.0,
                    a: 0.35,
                },
                None,
            )?;

            let red_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 1.0, g: 0.08, b: 0.12, a: 0.75 },
                None,
            )?;
            let green_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.08, g: 1.0, b: 0.25, a: 0.75 },
                None,
            )?;
            let blue_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.08, g: 0.45, b: 1.0, a: 0.75 },
                None,
            )?;

            // Dark outline brush for authentic cursor border
            let outline_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 0.04, g: 0.04, b: 0.04, a: 1.0 },
                None,
            )?;

            // Reusable dynamic brush for zero-allocation rendering of motion blur samples
            let dynamic_brush = d2d_context.CreateSolidColorBrush(
                &D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                None,
            )?;

            // Pre-bake classic Windows cursor arrow geometry once at origin (0, 0)
            let arrow_geometry = d2d_factory.CreatePathGeometry()?;
            let sink = arrow_geometry.Open()?;
            let arrow_pts = [
                Vector2 { X: 0.0, Y: 0.0 },       // Tip
                Vector2 { X: 0.0, Y: 16.5 },      // Left edge
                Vector2 { X: 4.5, Y: 12.5 },      // Inner notch
                Vector2 { X: 7.5, Y: 19.5 },      // Stem bottom left
                Vector2 { X: 10.0, Y: 18.2 },     // Stem bottom right
                Vector2 { X: 7.0, Y: 11.2 },      // Stem top right
                Vector2 { X: 12.5, Y: 11.2 },     // Right barb
            ];
            sink.BeginFigure(arrow_pts[0], D2D1_FIGURE_BEGIN_FILLED);
            sink.AddLines(&arrow_pts[1..]);
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;

            Ok(Self {
                _d3d_device: d3d_device,
                _d3d_context: d3d_context,
                swap_chain,
                _dcomp_device: dcomp_device,
                _dcomp_target: dcomp_target,
                _dcomp_visual: dcomp_visual,
                d2d_factory,
                d2d_context,
                target_bitmap: Some(target_bitmap),
                brush,
                glow_brush,
                outline_brush,
                red_brush,
                green_brush,
                blue_brush,
                dynamic_brush,
                arrow_geometry: arrow_geometry.into(),
                width,
                height,
                rainbow_phase: 0.0,
            })
        }
    }

    pub fn d2d_factory(&self) -> &ID2D1Factory1 {
        &self.d2d_factory
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        let width = width.max(1);
        let height = height.max(1);

        if self.width == width && self.height == height {
            return Ok(());
        }

        unsafe {
            self.d2d_context.SetTarget(None);
            self.target_bitmap = None;

            self.swap_chain.ResizeBuffers(
                2,
                width,
                height,
                DXGI_FORMAT_B8G8R8A8_UNORM,
                DXGI_SWAP_CHAIN_FLAG(0),
            )?;

            self.width = width;
            self.height = height;

            let surface: IDXGISurface = self.swap_chain.GetBuffer(0)?;
            let bitmap_props = D2D1_BITMAP_PROPERTIES1 {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
                bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                colorContext: ManuallyDrop::new(None),
            };

            let target_bitmap = self.d2d_context.CreateBitmapFromDxgiSurface(&surface, Some(&bitmap_props))?;
            self.d2d_context.SetTarget(&target_bitmap);
            self.target_bitmap = Some(target_bitmap);
        }

        Ok(())
    }

    pub fn render_frame(
        &mut self,
        tracker: &mut MouseTracker,
        config: &Config,
        origin_x: f32,
        origin_y: f32,
    ) -> Result<()> {
        unsafe {
            self.d2d_context.BeginDraw();
            self.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));

            self.rainbow_phase = (self.rainbow_phase + 1.2) % 360.0;
            let rgba = config.get_color_rgba(self.rainbow_phase);

            self.brush.SetColor(&D2D1_COLOR_F {
                r: rgba[0],
                g: rgba[1],
                b: rgba[2],
                a: rgba[3],
            });

            self.glow_brush.SetColor(&D2D1_COLOR_F {
                r: rgba[0],
                g: rgba[1],
                b: rgba[2],
                a: rgba[3] * 0.35,
            });

            let samples = tracker.get_motion_blur_samples(
                config.tail_duration_ms,
                config.min_velocity,
                config.max_velocity,
            );

            match config.mode {
                BlurMode::Optical => {
                    self.draw_optical_motion_blur(samples, origin_x, origin_y, rgba, config.opacity)?;
                }
                BlurMode::DirectionalSmear => {
                    self.draw_directional_smear(samples, origin_x, origin_y, rgba, config.opacity)?;
                }
                BlurMode::GhostShutter => {
                    self.draw_ghost_shutter(samples, origin_x, origin_y, rgba, config.afterimage_count)?;
                }
                BlurMode::NeonBloom => {
                    self.draw_neon_bloom_motion_blur(samples, origin_x, origin_y, rgba, config.glow_radius, config.opacity)?;
                }
                BlurMode::ParticleSparks => {
                    self.draw_optical_motion_blur(samples, origin_x, origin_y, rgba, config.opacity)?;

                    // Luminous kinetic micro-sparks on fast flicks
                    let particles = tracker.get_particles();
                    for p in particles {
                        let spark_alpha = (rgba[3] * p.life * config.opacity).clamp(0.0, 1.0);
                        self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                            r: (rgba[0] * 1.25).min(1.0),
                            g: (rgba[1] * 1.25).min(1.0),
                            b: (rgba[2] * 1.25).min(1.0),
                            a: spark_alpha,
                        });

                        let ellipse = D2D1_ELLIPSE {
                            point: Vector2 {
                                X: p.pos.x - origin_x,
                                Y: p.pos.y - origin_y,
                            },
                            radiusX: p.size,
                            radiusY: p.size,
                        };
                        self.d2d_context.FillEllipse(&ellipse, &self.dynamic_brush);
                    }
                }
                BlurMode::ChromaticAberration => {
                    self.draw_chromatic_motion_blur(samples, origin_x, origin_y, config.rgb_split_distance, config.opacity)?;
                }
                BlurMode::SpeedLines => {
                    self.draw_optical_motion_blur(samples, origin_x, origin_y, rgba, config.opacity)?;

                    let lines = tracker.generate_speed_lines(config.speed_lines_count);
                    for &(start, end, rel_alpha) in lines {
                        let line_alpha = rgba[3] * rel_alpha * config.opacity * 0.85;
                        self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                            r: rgba[0],
                            g: rgba[1],
                            b: rgba[2],
                            a: line_alpha,
                        });

                        let p0 = Vector2 { X: start.x - origin_x, Y: start.y - origin_y };
                        let p1 = Vector2 { X: end.x - origin_x, Y: end.y - origin_y };
                        self.d2d_context.DrawLine(p0, p1, &self.dynamic_brush, 2.0, None);
                    }
                }
                BlurMode::PlasmaArc => {
                    self.draw_optical_motion_blur(samples, origin_x, origin_y, rgba, config.opacity)?;

                    let arcs = tracker.generate_plasma_arcs(config.plasma_jitter);
                    for &(a, b, rel_alpha) in arcs {
                        let arc_alpha = (rgba[3] * rel_alpha * config.opacity).clamp(0.0, 1.0);
                        self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                            r: rgba[0],
                            g: rgba[1],
                            b: rgba[2],
                            a: arc_alpha,
                        });

                        let p0 = Vector2 { X: a.x - origin_x, Y: a.y - origin_y };
                        let p1 = Vector2 { X: b.x - origin_x, Y: b.y - origin_y };

                        // Outer electric discharge
                        self.d2d_context.DrawLine(p0, p1, &self.glow_brush, 3.5, None);
                        // Core plasma bolt
                        self.d2d_context.DrawLine(p0, p1, &self.dynamic_brush, 1.6, None);
                    }
                }
            }

            self.d2d_context.EndDraw(None, None)?;

            let sync_interval = if config.target_fps == 0 { 1 } else { 0 };
            let _ = self.swap_chain.Present(sync_interval, DXGI_PRESENT(0));
        }

        Ok(())
    }

    /// Authentic optical cursor motion blur with realistic dark outline & luminous fill
    unsafe fn draw_optical_motion_blur(
        &self,
        samples: &[(crate::trail::Vec2, crate::trail::Vec2, f32)],
        origin_x: f32,
        origin_y: f32,
        rgba: [f32; 4],
        opacity: f32,
    ) -> Result<()> {
        unsafe {
            for &(pos, _normal, rel_alpha) in samples {
                let sample_alpha = (rgba[3] * rel_alpha * opacity).clamp(0.0, 1.0);
                if sample_alpha < 0.003 {
                    continue;
                }

                let px = pos.x - origin_x;
                let py = pos.y - origin_y;

                let transform = windows_numerics::Matrix3x2::translation(px, py);
                self.d2d_context.SetTransform(&transform);

                // Pass 1: Crisp dark cursor border
                self.outline_brush.SetColor(&D2D1_COLOR_F {
                    r: 0.04,
                    g: 0.04,
                    b: 0.04,
                    a: (sample_alpha * 0.70).clamp(0.0, 1.0),
                });
                self.d2d_context.DrawGeometry(&self.arrow_geometry, &self.outline_brush, 1.5, None);

                // Pass 2: Luminous cursor interior (white or user color)
                self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: (sample_alpha * 0.88).clamp(0.0, 1.0),
                });
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.dynamic_brush, None);
            }

            self.d2d_context.SetTransform(&windows_numerics::Matrix3x2::identity());
        }
        Ok(())
    }

    /// Continuous directional extruded smear
    unsafe fn draw_directional_smear(
        &self,
        samples: &[(crate::trail::Vec2, crate::trail::Vec2, f32)],
        origin_x: f32,
        origin_y: f32,
        rgba: [f32; 4],
        opacity: f32,
    ) -> Result<()> {
        unsafe {
            for &(pos, _normal, rel_alpha) in samples {
                let sample_alpha = (rgba[3] * rel_alpha * opacity * 1.35).clamp(0.0, 1.0);
                if sample_alpha < 0.003 {
                    continue;
                }

                let px = pos.x - origin_x;
                let py = pos.y - origin_y;

                let transform = windows_numerics::Matrix3x2::translation(px, py);
                self.d2d_context.SetTransform(&transform);

                self.outline_brush.SetColor(&D2D1_COLOR_F {
                    r: 0.04,
                    g: 0.04,
                    b: 0.04,
                    a: (sample_alpha * 0.50).clamp(0.0, 1.0),
                });
                self.d2d_context.DrawGeometry(&self.arrow_geometry, &self.outline_brush, 1.4, None);

                self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: sample_alpha,
                });
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.dynamic_brush, None);
            }

            self.d2d_context.SetTransform(&windows_numerics::Matrix3x2::identity());
        }
        Ok(())
    }

    /// High-speed camera temporal shutter strobe exposures
    unsafe fn draw_ghost_shutter(
        &self,
        samples: &[(crate::trail::Vec2, crate::trail::Vec2, f32)],
        origin_x: f32,
        origin_y: f32,
        rgba: [f32; 4],
        count: usize,
    ) -> Result<()> {
        if samples.is_empty() {
            return Ok(());
        }

        unsafe {
            let stride = (samples.len() / count.clamp(2, 8)).max(1);
            for i in (0..samples.len()).step_by(stride) {
                let (pos, _normal, rel_alpha) = samples[i];
                let ghost_alpha = (rgba[3] * rel_alpha * 2.8).clamp(0.0, 0.75);
                if ghost_alpha < 0.02 {
                    continue;
                }

                let px = pos.x - origin_x;
                let py = pos.y - origin_y;

                let transform = windows_numerics::Matrix3x2::translation(px, py);
                self.d2d_context.SetTransform(&transform);

                self.outline_brush.SetColor(&D2D1_COLOR_F {
                    r: 0.04,
                    g: 0.04,
                    b: 0.04,
                    a: ghost_alpha * 0.75,
                });
                self.d2d_context.DrawGeometry(&self.arrow_geometry, &self.outline_brush, 1.5, None);

                self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: ghost_alpha,
                });
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.dynamic_brush, None);
            }

            self.d2d_context.SetTransform(&windows_numerics::Matrix3x2::identity());
        }
        Ok(())
    }

    /// Laser neon motion blur with soft ambient bloom
    unsafe fn draw_neon_bloom_motion_blur(
        &self,
        samples: &[(crate::trail::Vec2, crate::trail::Vec2, f32)],
        origin_x: f32,
        origin_y: f32,
        rgba: [f32; 4],
        glow_radius: f32,
        opacity: f32,
    ) -> Result<()> {
        unsafe {
            for &(pos, _normal, rel_alpha) in samples {
                let sample_alpha = (rgba[3] * rel_alpha * opacity).clamp(0.0, 1.0);
                if sample_alpha < 0.003 {
                    continue;
                }

                let px = pos.x - origin_x;
                let py = pos.y - origin_y;

                let transform = windows_numerics::Matrix3x2::translation(px, py);
                self.d2d_context.SetTransform(&transform);

                // Ambient glow aura
                self.glow_brush.SetColor(&D2D1_COLOR_F {
                    r: rgba[0],
                    g: rgba[1],
                    b: rgba[2],
                    a: (sample_alpha * 0.35).clamp(0.0, 1.0),
                });
                self.d2d_context.DrawGeometry(&self.arrow_geometry, &self.glow_brush, glow_radius * 1.8, None);

                // Laser core
                self.dynamic_brush.SetColor(&D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: (sample_alpha * 0.95).clamp(0.0, 1.0),
                });
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.dynamic_brush, None);
            }

            self.d2d_context.SetTransform(&windows_numerics::Matrix3x2::identity());
        }
        Ok(())
    }

    /// Prismatic RGB color separation motion blur
    unsafe fn draw_chromatic_motion_blur(
        &self,
        samples: &[(crate::trail::Vec2, crate::trail::Vec2, f32)],
        origin_x: f32,
        origin_y: f32,
        split_distance: f32,
        opacity: f32,
    ) -> Result<()> {
        unsafe {
            for &(pos, normal, rel_alpha) in samples {
                let sample_alpha = (rel_alpha * opacity).clamp(0.0, 1.0);
                if sample_alpha < 0.003 {
                    continue;
                }

                let px = pos.x - origin_x;
                let py = pos.y - origin_y;
                let d = split_distance * sample_alpha;

                // Red offset along +normal
                self.red_brush.SetColor(&D2D1_COLOR_F { r: 1.0, g: 0.08, b: 0.15, a: sample_alpha * 0.75 });
                let red_mat = windows_numerics::Matrix3x2::translation(px + normal.x * d, py + normal.y * d);
                self.d2d_context.SetTransform(&red_mat);
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.red_brush, None);

                // Green centered
                self.green_brush.SetColor(&D2D1_COLOR_F { r: 0.08, g: 1.0, b: 0.25, a: sample_alpha * 0.75 });
                let green_mat = windows_numerics::Matrix3x2::translation(px, py);
                self.d2d_context.SetTransform(&green_mat);
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.green_brush, None);

                // Blue offset along -normal
                self.blue_brush.SetColor(&D2D1_COLOR_F { r: 0.08, g: 0.45, b: 1.0, a: sample_alpha * 0.75 });
                let blue_mat = windows_numerics::Matrix3x2::translation(px - normal.x * d, py - normal.y * d);
                self.d2d_context.SetTransform(&blue_mat);
                self.d2d_context.FillGeometry(&self.arrow_geometry, &self.blue_brush, None);
            }

            self.d2d_context.SetTransform(&windows_numerics::Matrix3x2::identity());
        }
        Ok(())
    }

    pub fn clear(&mut self) -> Result<()> {
        unsafe {
            self.d2d_context.BeginDraw();
            self.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            self.d2d_context.EndDraw(None, None)?;
            let _ = self.swap_chain.Present(1, DXGI_PRESENT(0));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::OverlayWindow;

    #[test]
    fn test_renderer_initialization_and_frame() {
        let overlay = OverlayWindow::new().expect("OverlayWindow::new failed");
        let mut renderer = Renderer::new(overlay.hwnd, overlay.width as u32, overlay.height as u32)
            .expect("Renderer::new failed");
        let mut tracker = MouseTracker::new();
        let config = Config::default();
        tracker.record_position(100.0, 100.0, 32, 24.0);
        tracker.record_position(150.0, 150.0, 32, 24.0);
        let res = renderer.render_frame(&mut tracker, &config, overlay.x as f32, overlay.y as f32);
        assert!(res.is_ok(), "render_frame failed: {:?}", res);
        let clear_res = renderer.clear();
        assert!(clear_res.is_ok(), "clear failed: {:?}", clear_res);
    }
}
