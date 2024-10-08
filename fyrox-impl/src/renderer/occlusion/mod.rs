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

//! Full algorithm explained - <https://fyrox.rs/blog/post/tile-based-occlusion-culling/>

mod grid;
mod optimizer;

use crate::renderer::occlusion::grid::{GridCache, Visibility};
use crate::{
    core::{
        algebra::{Matrix4, Vector2, Vector3, Vector4},
        array_as_u8_slice,
        color::Color,
        math::{aabb::AxisAlignedBoundingBox, OptionRect, Rect, Vector3Ext},
        pool::Handle,
        ImmutableString,
    },
    graph::BaseSceneGraph,
    renderer::{
        debug_renderer,
        debug_renderer::DebugRenderer,
        framework::{
            error::FrameworkError,
            framebuffer::{
                Attachment, AttachmentKind, BlendParameters, CullFace, DrawParameters, FrameBuffer,
            },
            geometry_buffer::{GeometryBuffer, GeometryBufferKind},
            gpu_program::{GpuProgram, UniformLocation},
            gpu_texture::{
                Coordinate, GpuTexture, GpuTextureKind, MagnificationFilter, MinificationFilter,
                PixelKind, WrapMode,
            },
            state::{
                BlendEquation, BlendFactor, BlendFunc, BlendMode, ColorMask, CompareFunc,
                PipelineState,
            },
        },
        occlusion::optimizer::VisibilityBufferOptimizer,
        storage::MatrixStorage,
    },
    scene::{graph::Graph, mesh::surface::SurfaceData, node::Node},
};
use bytemuck::{Pod, Zeroable};
use std::{cell::RefCell, rc::Rc};

struct Shader {
    program: GpuProgram,
    view_projection: UniformLocation,
    tile_size: UniformLocation,
    tile_buffer: UniformLocation,
    frame_buffer_height: UniformLocation,
    matrices: UniformLocation,
}

impl Shader {
    fn new(state: &PipelineState) -> Result<Self, FrameworkError> {
        let fragment_source = include_str!("../shaders/visibility_fs.glsl");
        let vertex_source = include_str!("../shaders/visibility_vs.glsl");
        let program =
            GpuProgram::from_source(state, "VisibilityShader", vertex_source, fragment_source)?;
        Ok(Self {
            view_projection: program
                .uniform_location(state, &ImmutableString::new("viewProjection"))?,
            tile_size: program.uniform_location(state, &ImmutableString::new("tileSize"))?,
            frame_buffer_height: program
                .uniform_location(state, &ImmutableString::new("frameBufferHeight"))?,
            tile_buffer: program.uniform_location(state, &ImmutableString::new("tileBuffer"))?,
            matrices: program.uniform_location(state, &ImmutableString::new("matrices"))?,
            program,
        })
    }
}

pub struct OcclusionTester {
    framebuffer: FrameBuffer,
    visibility_mask: Rc<RefCell<GpuTexture>>,
    tile_buffer: Rc<RefCell<GpuTexture>>,
    frame_size: Vector2<usize>,
    shader: Shader,
    tile_size: usize,
    w_tiles: usize,
    h_tiles: usize,
    cube: GeometryBuffer,
    visibility_buffer_optimizer: VisibilityBufferOptimizer,
    matrix_storage: MatrixStorage,
    objects_to_test: Vec<Handle<Node>>,
    view_projection: Matrix4<f32>,
    observer_position: Vector3<f32>,
    pub grid_cache: GridCache,
    tiles: TileBuffer,
}

const MAX_BITS: usize = u32::BITS as usize;

#[derive(Default, Pod, Zeroable, Copy, Clone, Debug)]
#[repr(C)]
struct Tile {
    count: u32,
    objects: [u32; MAX_BITS],
}

impl Tile {
    fn add(&mut self, index: u32) {
        let count = self.count as usize;
        if count < self.objects.len() {
            self.objects[count] = index;
            self.count += 1;
        }
    }
}

#[derive(Default, Debug)]
struct TileBuffer {
    tiles: Vec<Tile>,
}

impl TileBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            tiles: vec![Default::default(); width * height],
        }
    }

    fn clear(&mut self) {
        for tile in self.tiles.iter_mut() {
            tile.count = 0;
        }
    }
}

fn screen_space_rect(
    aabb: AxisAlignedBoundingBox,
    view_projection: &Matrix4<f32>,
    viewport: &Rect<i32>,
) -> Rect<f32> {
    let mut rect_builder = OptionRect::default();
    for corner in aabb.corners() {
        let clip_space = view_projection * Vector4::new(corner.x, corner.y, corner.z, 1.0);
        let ndc_space = clip_space.xyz() / clip_space.w.abs();
        let mut normalized_screen_space =
            Vector2::new((ndc_space.x + 1.0) / 2.0, (1.0 - ndc_space.y) / 2.0);
        normalized_screen_space.x = normalized_screen_space.x.clamp(0.0, 1.0);
        normalized_screen_space.y = normalized_screen_space.y.clamp(0.0, 1.0);
        let screen_space_corner = Vector2::new(
            (normalized_screen_space.x * viewport.size.x as f32) + viewport.position.x as f32,
            (normalized_screen_space.y * viewport.size.y as f32) + viewport.position.y as f32,
        );

        rect_builder.push(screen_space_corner);
    }
    rect_builder.unwrap()
}

fn inflated_world_aabb(graph: &Graph, object: Handle<Node>) -> Option<AxisAlignedBoundingBox> {
    let mut aabb = graph
        .try_get(object)
        .map(|node_ref| node_ref.world_bounding_box())?;
    aabb.inflate(Vector3::repeat(0.01));
    Some(aabb)
}

impl OcclusionTester {
    pub fn new(
        state: &PipelineState,
        width: usize,
        height: usize,
        tile_size: usize,
    ) -> Result<Self, FrameworkError> {
        let mut depth_stencil_texture = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::D24S8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;
        depth_stencil_texture
            .bind_mut(state, 0)
            .set_wrap(Coordinate::S, WrapMode::ClampToEdge)
            .set_wrap(Coordinate::T, WrapMode::ClampToEdge);

        let visibility_mask = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle { width, height },
            PixelKind::RGBA8,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let w_tiles = width / tile_size + 1;
        let h_tiles = height / tile_size + 1;
        let tile_buffer = GpuTexture::new(
            state,
            GpuTextureKind::Rectangle {
                width: w_tiles * (MAX_BITS + 1),
                height: h_tiles,
            },
            PixelKind::R32UI,
            MinificationFilter::Nearest,
            MagnificationFilter::Nearest,
            1,
            None,
        )?;

        let depth_stencil = Rc::new(RefCell::new(depth_stencil_texture));
        let visibility_mask = Rc::new(RefCell::new(visibility_mask));
        let tile_buffer = Rc::new(RefCell::new(tile_buffer));

        Ok(Self {
            framebuffer: FrameBuffer::new(
                state,
                Some(Attachment {
                    kind: AttachmentKind::DepthStencil,
                    texture: depth_stencil,
                }),
                vec![Attachment {
                    kind: AttachmentKind::Color,
                    texture: visibility_mask.clone(),
                }],
            )?,
            visibility_mask,
            frame_size: Vector2::new(width, height),
            shader: Shader::new(state)?,
            tile_size,
            w_tiles,
            tile_buffer,
            h_tiles,
            cube: GeometryBuffer::from_surface_data(
                &SurfaceData::make_cube(Matrix4::identity()),
                GeometryBufferKind::StaticDraw,
                state,
            )?,
            visibility_buffer_optimizer: VisibilityBufferOptimizer::new(state, w_tiles, h_tiles)?,
            matrix_storage: MatrixStorage::new(state)?,
            objects_to_test: Default::default(),
            view_projection: Default::default(),
            observer_position: Default::default(),
            grid_cache: GridCache::new(Vector3::repeat(1)),
            tiles: TileBuffer::new(w_tiles, h_tiles),
        })
    }

    pub fn try_query_visibility_results(&mut self, state: &PipelineState, graph: &Graph) {
        let Some(visibility_buffer) = self.visibility_buffer_optimizer.read_visibility_mask(state)
        else {
            return;
        };

        let mut objects_visibility = vec![false; self.objects_to_test.len()];
        for y in 0..self.h_tiles {
            let img_y = self.h_tiles.saturating_sub(1) - y;
            let tile_offset = y * self.w_tiles;
            let img_offset = img_y * self.w_tiles;
            for x in 0..self.w_tiles {
                let tile = &self.tiles.tiles[tile_offset + x];
                let bits = visibility_buffer[img_offset + x];
                let count = tile.count as usize;
                for bit in 0..count {
                    let object_index = tile.objects[bit];
                    let visibility = &mut objects_visibility[object_index as usize];
                    let mask = 1 << bit;
                    let is_visible = (bits & mask) == mask;
                    if is_visible {
                        *visibility = true;
                    }
                }
            }
        }

        let cell = self.grid_cache.get_or_insert_cell(self.observer_position);
        for (obj, vis) in self.objects_to_test.iter().zip(objects_visibility.iter()) {
            cell.mark(*obj, (*vis).into());
        }

        for (object, visibility) in cell.iter_mut() {
            let Some(aabb) = inflated_world_aabb(graph, *object) else {
                continue;
            };
            if aabb.is_contains_point(self.observer_position) {
                *visibility = Visibility::Visible;
            }
        }
    }

    fn screen_space_to_tile_space(&self, pos: Vector2<f32>) -> Vector2<usize> {
        let x = (pos.x / (self.tile_size as f32)) as usize;
        let y = (pos.y / (self.tile_size as f32)) as usize;
        Vector2::new(x, y)
    }

    fn prepare_tiles(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        viewport: &Rect<i32>,
        debug_renderer: Option<&mut DebugRenderer>,
    ) -> Result<(), FrameworkError> {
        self.tiles.clear();

        let mut lines = Vec::new();
        for (object_index, object) in self.objects_to_test.iter().enumerate() {
            let object_index = object_index as u32;
            let Some(node_ref) = graph.try_get(*object) else {
                continue;
            };

            let aabb = node_ref.world_bounding_box();
            let rect = screen_space_rect(aabb, &self.view_projection, viewport);

            if debug_renderer.is_some() {
                debug_renderer::draw_rect(&rect, &mut lines, Color::WHITE);
            }

            let min = self.screen_space_to_tile_space(rect.left_top_corner());
            let max = self.screen_space_to_tile_space(rect.right_bottom_corner());
            for y in min.y..=max.y {
                let offset = y * self.w_tiles;
                for x in min.x..=max.x {
                    self.tiles.tiles[offset + x].add(object_index);
                }
            }
        }

        if let Some(debug_renderer) = debug_renderer {
            for (tile_index, tile) in self.tiles.tiles.iter().enumerate() {
                let x = (tile_index % self.w_tiles) * self.tile_size;
                let y = (tile_index / self.w_tiles) * self.tile_size;
                let bounds = Rect::new(
                    x as f32,
                    y as f32,
                    self.tile_size as f32,
                    self.tile_size as f32,
                );

                debug_renderer::draw_rect(
                    &bounds,
                    &mut lines,
                    Color::COLORS[tile.objects.len() + 2],
                );
            }

            debug_renderer.set_lines(state, &lines);
        }

        self.tile_buffer.borrow_mut().bind_mut(state, 0).set_data(
            GpuTextureKind::Rectangle {
                width: self.w_tiles * (MAX_BITS + 1),
                height: self.h_tiles,
            },
            PixelKind::R32UI,
            1,
            Some(array_as_u8_slice(self.tiles.tiles.as_slice())),
        )?;

        Ok(())
    }

    fn upload_data<'a>(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        objects_to_test: impl Iterator<Item = &'a Handle<Node>>,
        prev_framebuffer: &FrameBuffer,
        observer_position: Vector3<f32>,
        view_projection: Matrix4<f32>,
    ) {
        self.view_projection = view_projection;
        self.observer_position = observer_position;
        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        state.blit_framebuffer(
            prev_framebuffer.id(),
            self.framebuffer.id(),
            0,
            0,
            w,
            h,
            0,
            0,
            w,
            h,
            false,
            true,
            false,
        );

        self.objects_to_test.clear();
        if let Some(cell) = self.grid_cache.cell(self.observer_position) {
            for object in objects_to_test {
                if cell.needs_occlusion_query(*object) {
                    self.objects_to_test.push(*object);
                }
            }
        }

        self.objects_to_test.sort_unstable_by_key(|a| {
            (graph[*a].global_position().sqr_distance(&observer_position) * 1000.0) as u64
        });
    }

    pub fn try_run_visibility_test<'a>(
        &mut self,
        state: &PipelineState,
        graph: &Graph,
        debug_renderer: Option<&mut DebugRenderer>,
        unit_quad: &GeometryBuffer,
        objects_to_test: impl Iterator<Item = &'a Handle<Node>>,
        prev_framebuffer: &FrameBuffer,
        observer_position: Vector3<f32>,
        view_projection: Matrix4<f32>,
    ) -> Result<(), FrameworkError> {
        if self.visibility_buffer_optimizer.is_reading_from_gpu() {
            return Ok(());
        }

        self.upload_data(
            state,
            graph,
            objects_to_test,
            prev_framebuffer,
            observer_position,
            view_projection,
        );

        let w = self.frame_size.x as i32;
        let h = self.frame_size.y as i32;
        let viewport = Rect::new(0, 0, w, h);

        self.framebuffer
            .clear(state, viewport, Some(Color::TRANSPARENT), None, None);

        self.prepare_tiles(state, graph, &viewport, debug_renderer)?;

        self.matrix_storage.upload(
            state,
            self.objects_to_test.iter().filter_map(|h| {
                let aabb = inflated_world_aabb(graph, *h)?;
                let s = aabb.max - aabb.min;
                Some(Matrix4::new_translation(&aabb.center()) * Matrix4::new_nonuniform_scaling(&s))
            }),
            0,
        )?;

        state.set_depth_func(CompareFunc::LessOrEqual);
        let shader = &self.shader;
        self.framebuffer.draw_instances(
            self.objects_to_test.len(),
            &self.cube,
            state,
            viewport,
            &self.shader.program,
            &DrawParameters {
                cull_face: Some(CullFace::Back),
                color_write: ColorMask::all(true),
                depth_write: false,
                stencil_test: None,
                depth_test: true,
                blend: Some(BlendParameters {
                    func: BlendFunc::new(BlendFactor::One, BlendFactor::One),
                    equation: BlendEquation {
                        rgb: BlendMode::Add,
                        alpha: BlendMode::Add,
                    },
                }),
                stencil_op: Default::default(),
            },
            |mut program_binding| {
                program_binding
                    .set_texture(&shader.tile_buffer, &self.tile_buffer)
                    .set_texture(&shader.matrices, self.matrix_storage.texture())
                    .set_i32(&shader.tile_size, self.tile_size as i32)
                    .set_f32(&shader.frame_buffer_height, self.frame_size.y as f32)
                    .set_matrix4(&shader.view_projection, &self.view_projection);
            },
        );

        self.visibility_buffer_optimizer.optimize(
            state,
            &self.visibility_mask,
            unit_quad,
            self.tile_size as i32,
        )?;

        Ok(())
    }
}
