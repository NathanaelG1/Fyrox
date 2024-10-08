// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::renderer::framework::geometry_buffer::DrawCallStatistics;
use fyrox_core::instant;
use std::fmt::{Display, Formatter};
use std::ops::AddAssign;

/// Graphics pipeline statistics.
#[derive(Debug, Default, Copy, Clone)]
pub struct PipelineStatistics {
    /// Total amount of texture that was bound to the pipeline during the rendering.
    pub texture_binding_changes: usize,
    /// Total amount of VBOs was bound to the pipeline during the rendering.
    pub vbo_binding_changes: usize,
    /// Total amount of VAOs was bound to the pipeline during the rendering.
    pub vao_binding_changes: usize,
    /// Total amount of blending state changed in the pipeline during the rendering.
    pub blend_state_changes: usize,
    /// Total amount of frame buffers was used during the rendering.
    pub framebuffer_binding_changes: usize,
    /// Total amount of programs was used in the pipeline during the rendering.
    pub program_binding_changes: usize,
}

impl std::ops::AddAssign for PipelineStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.texture_binding_changes += rhs.texture_binding_changes;
        self.vbo_binding_changes += rhs.vbo_binding_changes;
        self.vao_binding_changes += rhs.vao_binding_changes;
        self.blend_state_changes += rhs.blend_state_changes;
        self.framebuffer_binding_changes += rhs.framebuffer_binding_changes;
        self.program_binding_changes += rhs.program_binding_changes;
    }
}

impl std::ops::Sub for PipelineStatistics {
    type Output = PipelineStatistics;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            texture_binding_changes: self.texture_binding_changes - rhs.texture_binding_changes,
            vbo_binding_changes: self.vbo_binding_changes - rhs.vbo_binding_changes,
            vao_binding_changes: self.vao_binding_changes - rhs.vao_binding_changes,
            blend_state_changes: self.blend_state_changes - rhs.blend_state_changes,
            framebuffer_binding_changes: self.framebuffer_binding_changes
                - rhs.framebuffer_binding_changes,
            program_binding_changes: self.program_binding_changes - rhs.program_binding_changes,
        }
    }
}

impl Display for PipelineStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pipeline state changes:\n\
            \tTextures: {},\n\
            \tVBO: {},\n\
            \tVAO: {},\n\
            \tFBO: {},\n\
            \tShaders: {},\n\
            \tBlend: {}",
            self.texture_binding_changes,
            self.vbo_binding_changes,
            self.vao_binding_changes,
            self.framebuffer_binding_changes,
            self.program_binding_changes,
            self.blend_state_changes
        )
    }
}

/// Lighting statistics.
#[derive(Debug, Copy, Clone, Default)]
pub struct LightingStatistics {
    /// How many point lights were rendered.
    pub point_lights_rendered: usize,
    /// How many point light shadow maps were rendered.
    pub point_shadow_maps_rendered: usize,
    /// How many cascaded shadow maps were rendered.
    pub csm_rendered: usize,
    /// How many spot lights were rendered.
    pub spot_lights_rendered: usize,
    /// How many spot light shadow maps were rendered.
    pub spot_shadow_maps_rendered: usize,
    /// How many directional lights were rendered.
    pub directional_lights_rendered: usize,
}

impl AddAssign for LightingStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.point_lights_rendered += rhs.point_lights_rendered;
        self.point_shadow_maps_rendered += rhs.point_shadow_maps_rendered;
        self.spot_lights_rendered += rhs.spot_lights_rendered;
        self.spot_shadow_maps_rendered += rhs.spot_shadow_maps_rendered;
        self.directional_lights_rendered += rhs.directional_lights_rendered;
        self.csm_rendered += rhs.csm_rendered;
    }
}

impl Display for LightingStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Lighting Statistics:\n\
            \tPoint Lights: {}\n\
            \tSpot Lights: {}\n\
            \tDirectional Lights: {}\n\
            \tPoint Shadow Maps: {}\n\
            \tSpot Shadow Maps: {}\n\
            \tSpot Shadow Maps: {}\n",
            self.point_lights_rendered,
            self.spot_lights_rendered,
            self.directional_lights_rendered,
            self.point_shadow_maps_rendered,
            self.spot_shadow_maps_rendered,
            self.csm_rendered
        )
    }
}

/// Renderer statistics for a scene.
#[derive(Debug, Copy, Clone, Default)]
pub struct SceneStatistics {
    /// Shows how many pipeline state changes was made during scene rendering.
    pub pipeline: PipelineStatistics,
    /// Shows how many lights and shadow maps were rendered.
    pub lighting: LightingStatistics,
    /// Shows how many draw calls was made and how many triangles were rendered.
    pub geometry: RenderPassStatistics,
}

impl Display for SceneStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n\
            {}\n\
            {}\n",
            self.geometry, self.lighting, self.pipeline
        )
    }
}

impl AddAssign<DrawCallStatistics> for SceneStatistics {
    fn add_assign(&mut self, rhs: DrawCallStatistics) {
        self.geometry += rhs;
    }
}

impl AddAssign<PipelineStatistics> for SceneStatistics {
    fn add_assign(&mut self, rhs: PipelineStatistics) {
        self.pipeline += rhs;
    }
}

impl AddAssign<RenderPassStatistics> for SceneStatistics {
    fn add_assign(&mut self, rhs: RenderPassStatistics) {
        self.geometry += rhs;
    }
}

impl AddAssign<LightingStatistics> for SceneStatistics {
    fn add_assign(&mut self, rhs: LightingStatistics) {
        self.lighting += rhs;
    }
}

/// Renderer statistics for one frame, also includes current frames per second
/// amount.
#[derive(Debug, Copy, Clone)]
pub struct Statistics {
    /// Shows how many pipeline state changes was made per frame.
    pub pipeline: PipelineStatistics,
    /// Shows how many lights and shadow maps were rendered.
    pub lighting: LightingStatistics,
    /// Shows how many draw calls was made and how many triangles were rendered.
    pub geometry: RenderPassStatistics,
    /// Real time consumed to render frame. Time given in **seconds**.
    pub pure_frame_time: f32,
    /// Total time renderer took to process single frame, usually includes
    /// time renderer spend to wait to buffers swap (can include vsync).
    /// Time given in **seconds**.
    pub capped_frame_time: f32,
    /// Total amount of frames been rendered in one second.
    pub frames_per_second: usize,
    /// Total amount of textures in the textures cache.
    pub texture_cache_size: usize,
    /// Total amount of vertex+index buffers pairs in the geometry cache.
    pub geometry_cache_size: usize,
    /// Total amount of shaders in the shaders cache.
    pub shader_cache_size: usize,
    pub(super) frame_counter: usize,
    pub(super) frame_start_time: instant::Instant,
    pub(super) last_fps_commit_time: instant::Instant,
}

impl std::ops::AddAssign<SceneStatistics> for Statistics {
    fn add_assign(&mut self, rhs: SceneStatistics) {
        self.pipeline += rhs.pipeline;
        self.lighting += rhs.lighting;
        self.geometry += rhs.geometry;
    }
}

impl Display for Statistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fps = self.frames_per_second;
        let pure_frame_time = self.pure_frame_time * 1000.0;
        let capped_frame_time = self.capped_frame_time * 1000.0;
        let geometry_stats = &self.geometry;
        let lighting_stats = &self.lighting;
        let pipeline_stats = &self.pipeline;
        let texture_cache_size = self.texture_cache_size;
        let geometry_cache_size = self.geometry_cache_size;
        let shader_cache_size = self.shader_cache_size;
        write!(
            f,
            "FPS: {fps}\n\
            Pure Frame Time: {pure_frame_time:.2} ms\n\
            Capped Frame Time: {capped_frame_time:.2} ms\n\
            {geometry_stats}\n\
            {lighting_stats}\n\
            {pipeline_stats}\n\
            Texture Cache Size: {texture_cache_size}\n\
            Geometry Cache Size: {geometry_cache_size}\n\
            Shader Cache Size: {shader_cache_size}",
        )
    }
}

/// GPU statistics for single frame.
#[derive(Debug, Copy, Clone, Default)]
pub struct RenderPassStatistics {
    /// Amount of draw calls per frame - lower the better.
    pub draw_calls: usize,
    /// Amount of triangles per frame.
    pub triangles_rendered: usize,
}

impl Display for RenderPassStatistics {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Draw Calls: {}\n\
            Triangles Rendered: {}",
            self.draw_calls, self.triangles_rendered
        )
    }
}

impl std::ops::AddAssign for RenderPassStatistics {
    fn add_assign(&mut self, rhs: Self) {
        self.draw_calls += rhs.draw_calls;
        self.triangles_rendered += rhs.triangles_rendered;
    }
}

impl std::ops::AddAssign<DrawCallStatistics> for RenderPassStatistics {
    fn add_assign(&mut self, rhs: DrawCallStatistics) {
        self.draw_calls += 1;
        self.triangles_rendered += rhs.triangles;
    }
}

impl std::ops::AddAssign<RenderPassStatistics> for Statistics {
    fn add_assign(&mut self, rhs: RenderPassStatistics) {
        self.geometry += rhs;
    }
}
