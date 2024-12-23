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

use brush::TileMapBrushResource;
use fyrox::{
    core::algebra::Vector2,
    gui::{
        grid::SizeMode,
        stack_panel::StackPanelBuilder,
        text::{TextBuilder, TextMessage},
        window::Window,
    },
    scene::tilemap::{tileset::TileSet, TileResource, *},
};

use crate::{
    asset::item::AssetItem,
    fyrox::{
        core::color::Color,
        core::pool::Handle,
        graph::{BaseSceneGraph, SceneGraph},
        gui::{
            border::BorderBuilder,
            brush::Brush,
            button::{Button, ButtonMessage},
            decorator::DecoratorMessage,
            grid::{Column, GridBuilder, Row},
            message::{MessageDirection, UiMessage},
            widget::{WidgetBuilder, WidgetMessage},
            window::{WindowBuilder, WindowMessage, WindowTitle},
            wrap_panel::WrapPanelBuilder,
            BuildContext, HorizontalAlignment, Orientation, Thickness, UiNode, UserInterface,
            VerticalAlignment,
        },
        scene::{
            node::Node,
            tilemap::{brush::TileMapBrush, TileMap},
        },
    },
    message::MessageSender,
    plugins::tilemap::{
        palette::{PaletteMessage, PaletteWidgetBuilder},
        DrawingMode,
    },
    scene::container::EditorSceneEntry,
};

use super::*;

const DEFAULT_PAGE: Vector2<i32> = Vector2::new(0, 0);

fn highlight_tool_button(button: Handle<UiNode>, highlight: bool, ui: &UserInterface) {
    let decorator = *ui.try_get_of_type::<Button>(button).unwrap().decorator;
    ui.send_message(DecoratorMessage::select(
        decorator,
        MessageDirection::ToWidget,
        highlight,
    ));
}

fn make_resource_chooser(
    ctx: &mut BuildContext,
    text: &str,
    tooltip: &str,
    tab_index: Option<usize>,
    column: usize,
) -> Handle<UiNode> {
    ButtonBuilder::new(
        WidgetBuilder::new()
            .on_column(column)
            .with_tab_index(tab_index)
            .with_tooltip(make_simple_tooltip(ctx, tooltip))
            .with_margin(Thickness::uniform(1.0)),
    )
    .with_back(
        DecoratorBuilder::new(
            BorderBuilder::new(
                WidgetBuilder::new().with_foreground(ctx.style.property(Style::BRUSH_DARKER)),
            )
            .with_pad_by_corner_radius(false)
            .with_corner_radius((4.0).into())
            .with_stroke_thickness(Thickness::uniform(1.0).into()),
        )
        .with_selected_brush(ctx.style.property(Style::BRUSH_BRIGHT_BLUE))
        .with_normal_brush(ctx.style.property(Style::BRUSH_LIGHT))
        .with_hover_brush(ctx.style.property(Style::BRUSH_LIGHTER))
        .with_pressed_brush(ctx.style.property(Style::BRUSH_LIGHTEST))
        .build(ctx),
    )
    .with_text(text)
    .build(ctx)
}

pub struct TileMapPanel {
    pub state: TileDrawStateRef,
    pub tile_resource: TileResource,
    pub window: Handle<UiNode>,
    brush: Option<TileMapBrushResource>,
    tile_set_name: Handle<UiNode>,
    preview: Handle<UiNode>,
    pages: Handle<UiNode>,
    palette: Handle<UiNode>,
    brush_button: Handle<UiNode>,
    tile_set_button: Handle<UiNode>,
    draw_button: Handle<UiNode>,
    erase_button: Handle<UiNode>,
    flood_fill_button: Handle<UiNode>,
    pick_button: Handle<UiNode>,
    rect_fill_button: Handle<UiNode>,
    nine_slice_button: Handle<UiNode>,
    line_button: Handle<UiNode>,
    random_button: Handle<UiNode>,
    left_button: Handle<UiNode>,
    right_button: Handle<UiNode>,
    flip_x_button: Handle<UiNode>,
    flip_y_button: Handle<UiNode>,
}

impl TileMapPanel {
    pub fn new(ctx: &mut BuildContext, state: TileDrawStateRef, sender: MessageSender) -> Self {
        let tile_set_name = TextBuilder::new(WidgetBuilder::new().on_row(0)).build(ctx);
        let preview = PanelPreviewBuilder::new(
            WidgetBuilder::new()
                .with_margin(Thickness::uniform(1.0))
                .with_min_size(Vector2::new(80.0, 100.0)),
            state.clone(),
        )
        .build(ctx);
        let pages = PaletteWidgetBuilder::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            sender.clone(),
            state.clone(),
        )
        .with_page(DEFAULT_PAGE)
        .with_kind(TilePaletteStage::Pages)
        .build(ctx);
        let palette = PaletteWidgetBuilder::new(
            WidgetBuilder::new().with_margin(Thickness::uniform(1.0)),
            sender.clone(),
            state.clone(),
        )
        .with_page(DEFAULT_PAGE)
        .with_kind(TilePaletteStage::Tiles)
        .build(ctx);
        let preview_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_column(1)
                .with_foreground(Brush::Solid(Color::BLACK).into())
                .with_child(preview),
        )
        .build(ctx);
        let pages_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(2)
                .with_margin(Thickness::uniform(2.0))
                .with_foreground(Brush::Solid(Color::BLACK).into())
                .with_child(pages),
        )
        .build(ctx);
        let palette_frame = BorderBuilder::new(
            WidgetBuilder::new()
                .on_row(3)
                .with_margin(Thickness::uniform(2.0))
                .with_foreground(Brush::Solid(Color::BLACK).into())
                .with_child(palette),
        )
        .build(ctx);

        let brush_button = make_resource_chooser(ctx, "Brush", "Draw tiles from a brush.", None, 0);
        let tile_set_button =
            make_resource_chooser(ctx, "Tile Set", "Draw tiles from the tile set.", None, 1);
        let resource_selector = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(brush_button)
                .with_child(tile_set_button),
        )
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .add_row(Row::auto())
        .build(ctx);

        let width = 20.0;
        let height = 20.0;
        let draw_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            BRUSH_IMAGE.clone(),
            "Draw with active brush.",
            Some(0),
        );
        let erase_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            ERASER_IMAGE.clone(),
            "Erase with active brush.",
            Some(1),
        );
        let flood_fill_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FILL_IMAGE.clone(),
            "Flood fill with tiles from current brush.",
            Some(2),
        );
        let pick_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            PICK_IMAGE.clone(),
            "Pick tiles for drawing from the tile map.",
            Some(3),
        );
        let rect_fill_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            RECT_FILL_IMAGE.clone(),
            "Fill the rectangle using the current brush.",
            Some(4),
        );
        let nine_slice_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            NINE_SLICE_IMAGE.clone(),
            "Draw rectangles with fixed corners, but stretchable sides.",
            Some(5),
        );
        let line_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            LINE_IMAGE.clone(),
            "Draw a line using tiles from the given brush.",
            Some(6),
        );
        let left_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_LEFT_IMAGE.clone(),
            "Rotate left 90 degrees.",
            Some(7),
        );
        let right_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            TURN_RIGHT_IMAGE.clone(),
            "Rotate right 90 degrees.",
            Some(8),
        );
        let flip_x_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_X_IMAGE.clone(),
            "Flip along x axis.",
            Some(9),
        );
        let flip_y_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            FLIP_Y_IMAGE.clone(),
            "Flip along y axis.",
            Some(10),
        );
        let random_button = make_drawing_mode_button(
            ctx,
            width,
            height,
            RANDOM_IMAGE.clone(),
            "Toggle random fill mode.",
            Some(11),
        );

        let drawing_modes_panel = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(draw_button)
                .with_child(erase_button)
                .with_child(flood_fill_button)
                .with_child(pick_button)
                .with_child(rect_fill_button)
                .with_child(nine_slice_button)
                .with_child(line_button),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);
        let modifiers_panel = WrapPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(left_button)
                .with_child(right_button)
                .with_child(flip_x_button)
                .with_child(flip_y_button)
                .with_child(random_button),
        )
        .with_orientation(Orientation::Horizontal)
        .build(ctx);

        let toolbar = StackPanelBuilder::new(
            WidgetBuilder::new()
                .with_child(resource_selector)
                .with_child(drawing_modes_panel)
                .with_child(modifiers_panel),
        )
        .build(ctx);

        let header = GridBuilder::new(
            WidgetBuilder::new()
                .on_row(1)
                .with_child(toolbar)
                .with_child(preview_frame),
        )
        .add_row(Row::auto())
        .add_column(Column::stretch())
        .add_column(Column::stretch())
        .build(ctx);

        let content = GridBuilder::new(
            WidgetBuilder::new()
                .with_child(tile_set_name)
                .with_child(header)
                .with_child(pages_frame)
                .with_child(palette_frame),
        )
        .add_row(Row::auto())
        .add_row(Row::auto())
        .add_row(Row::stretch())
        .add_row(Row::generic(SizeMode::Stretch, 200.0))
        .add_column(Column::stretch())
        .build(ctx);

        let window = WindowBuilder::new(WidgetBuilder::new().with_width(400.0).with_height(600.0))
            .open(false)
            .with_title(WindowTitle::text("Tile Map Control Panel"))
            .with_content(content)
            .build(ctx);

        Self {
            state,
            brush: None,
            tile_resource: TileResource::Empty,
            window,
            tile_set_name,
            preview,
            pages,
            palette,
            brush_button,
            tile_set_button,
            draw_button,
            erase_button,
            flood_fill_button,
            pick_button,
            rect_fill_button,
            nine_slice_button,
            line_button,
            left_button,
            right_button,
            flip_x_button,
            flip_y_button,
            random_button,
        }
    }

    pub fn align(&self, relative_to: Handle<UiNode>, ui: &UserInterface) {
        if ui.node(self.window).visibility() {
            ui.send_message(WidgetMessage::align(
                self.window,
                MessageDirection::ToWidget,
                relative_to,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
            ));
            ui.send_message(WidgetMessage::topmost(
                self.window,
                MessageDirection::ToWidget,
            ));
            ui.send_message(WidgetMessage::focus(
                ui.node(self.window).cast::<Window>().unwrap().content,
                MessageDirection::ToWidget,
            ));
        } else {
            ui.send_message(WindowMessage::open_and_align(
                self.window,
                MessageDirection::ToWidget,
                relative_to,
                HorizontalAlignment::Right,
                VerticalAlignment::Top,
                Thickness::uniform(2.0),
                false,
                true,
            ));
        }
    }

    pub fn to_top(&self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::topmost(
            self.window,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::focus(
            ui.node(self.window).cast::<Window>().unwrap().content,
            MessageDirection::ToWidget,
        ));
        ui.send_message(WidgetMessage::visibility(
            self.window,
            MessageDirection::ToWidget,
            true,
        ));
    }

    pub fn destroy(self, ui: &UserInterface) {
        ui.send_message(WidgetMessage::remove(
            self.window,
            MessageDirection::ToWidget,
        ));
    }

    pub fn set_resource(&mut self, resource: TileResource, ui: &mut UserInterface) {
        // Update the current brush based upon the new resource.
        match &resource {
            // An empty resource has no brush.
            TileResource::Empty => self.brush = None,
            // A tile set has no brush, but if this is the tile set of our current brush
            // then keep the current brush since it meant to be used with given resource.
            TileResource::TileSet(tile_set) => {
                if let Some(brush) = &self.brush {
                    if brush.data_ref().tile_set.as_ref() != Some(tile_set) {
                        self.brush = None;
                    }
                }
            }
            // We are being given a brush, so that becomes our brush.
            TileResource::Brush(brush) => {
                self.brush = Some(brush.clone());
            }
        }
        self.tile_resource = resource.clone();
        self.sync_to_model(ui);
        self.send_tile_resource(ui);
    }

    pub fn switch_to_brush(&mut self, ui: &mut UserInterface) {
        if let Some(brush) = &self.brush {
            self.tile_resource = TileResource::Brush(brush.clone());
            self.sync_to_model(ui);
            self.send_tile_resource(ui);
        }
    }

    pub fn switch_to_tile_set(&mut self, ui: &mut UserInterface) {
        let Some(brush) = &self.brush else {
            return;
        };
        let brush = brush.data_ref();
        let Some(tile_set) = brush.tile_set.clone() else {
            return;
        };
        self.tile_resource = TileResource::TileSet(tile_set);
        self.sync_to_model(ui);
        self.send_tile_resource(ui);
    }

    fn send_tile_resource(&self, ui: &mut UserInterface) {
        ui.send_message(PaletteMessage::set_page(
            self.pages,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(DEFAULT_PAGE),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.palette,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(DEFAULT_PAGE),
        ));
    }

    pub fn is_brush(&self) -> bool {
        self.tile_resource.is_brush()
    }

    pub fn is_tile_set(&self) -> bool {
        self.tile_resource.is_tile_set()
    }

    pub fn has_brush(&self) -> bool {
        self.brush.is_some()
    }

    pub fn set_focus(&mut self, handle: TileDefinitionHandle, ui: &mut UserInterface) {
        let mut state = self.state.lock_mut("set_focus");
        state.selection.source = SelectionSource::Widget(self.palette);
        let sel = state.selection_positions_mut();
        sel.clear();
        sel.insert(handle.tile());
        ui.send_message(PaletteMessage::set_page(
            self.pages,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::set_page(
            self.palette,
            MessageDirection::ToWidget,
            self.tile_resource.clone(),
            Some(handle.page()),
        ));
        ui.send_message(PaletteMessage::center(
            self.pages,
            MessageDirection::ToWidget,
            handle.page(),
        ));
        ui.send_message(PaletteMessage::center(
            self.palette,
            MessageDirection::ToWidget,
            handle.tile(),
        ));
        ui.send_message(PaletteMessage::select_one(
            self.palette,
            MessageDirection::ToWidget,
            handle.tile(),
        ));
    }

    pub fn set_visibility(&self, visible: bool, ui: &mut UserInterface) {
        ui.send_message(WidgetMessage::visibility(
            self.window,
            MessageDirection::ToWidget,
            visible,
        ));
    }

    fn handle_button(&mut self, button: Handle<UiNode>, ui: &mut UserInterface) {
        if button == self.draw_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::Draw;
        } else if button == self.erase_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::Erase;
        } else if button == self.flood_fill_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::FloodFill;
        } else if button == self.pick_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::Pick;
        } else if button == self.rect_fill_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::RectFill;
        } else if button == self.nine_slice_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::NineSlice;
        } else if button == self.line_button {
            self.state.lock_mut("tool button").drawing_mode = DrawingMode::Line;
        } else if button == self.random_button {
            let mut state = self.state.lock_mut("tool_button");
            state.random_mode = !state.random_mode;
        } else if button == self.left_button {
            self.state.lock_mut("tool button").stamp.rotate(1);
        } else if button == self.right_button {
            self.state.lock_mut("tool_button").stamp.rotate(-1);
        } else if button == self.flip_x_button {
            self.state.lock_mut("tool_button").stamp.x_flip();
        } else if button == self.flip_y_button {
            self.state.lock_mut("tool_button").stamp.y_flip();
        } else if button == self.brush_button {
            self.switch_to_brush(ui);
        } else if button == self.tile_set_button {
            self.switch_to_tile_set(ui);
        }
    }

    pub fn handle_ui_message(
        mut self,
        message: &UiMessage,
        ui: &mut UserInterface,
        _tile_map_handle: Handle<Node>,
        _tile_map: Option<&TileMap>,
        _sender: &MessageSender,
        _editor_scene: Option<&mut EditorSceneEntry>,
    ) -> Option<Self> {
        if let Some(WindowMessage::Close) = message.data() {
            if message.destination() == self.window {
                self.destroy(ui);
                return None;
            }
        }
        if let Some(ButtonMessage::Click) = message.data() {
            self.handle_button(message.destination(), ui);
        } else if let Some(PaletteMessage::SetPage { .. }) = message.data() {
            if message.destination() == self.pages
                && message.direction() == MessageDirection::FromWidget
            {
                ui.send_message(
                    message
                        .clone()
                        .with_destination(self.palette)
                        .with_direction(MessageDirection::ToWidget),
                );
            }
        } else if let Some(WidgetMessage::Drop(dropped)) = message.data() {
            if ui.is_node_child_of(message.destination(), self.window) {
                let tile_resource = if let Some(item) = ui.node(*dropped).cast::<AssetItem>() {
                    if let Some(brush) = item.resource::<TileMapBrush>() {
                        Some(TileResource::Brush(brush))
                    } else {
                        item.resource::<TileSet>().map(TileResource::TileSet)
                    }
                } else {
                    None
                };
                if let Some(tile_resource) = tile_resource {
                    self.set_resource(tile_resource, ui);
                }
            }
        }
        Some(self)
    }

    pub fn sync_to_model(&self, ui: &mut UserInterface) {
        let name = if let Some(path) = self.tile_resource.path() {
            path.to_string_lossy().into_owned()
        } else {
            "".into()
        };
        ui.send_message(TextMessage::text(
            self.tile_set_name,
            MessageDirection::ToWidget,
            name,
        ));
        highlight_tool_button(self.brush_button, self.tile_resource.is_brush(), ui);
        highlight_tool_button(self.tile_set_button, self.tile_resource.is_tile_set(), ui);
        ui.send_message(WidgetMessage::enabled(
            self.brush_button,
            MessageDirection::ToWidget,
            self.brush.is_some(),
        ));
        let has_tile_set = match &self.tile_resource {
            TileResource::Empty => false,
            TileResource::TileSet(_) => true,
            TileResource::Brush(brush) => brush.data_ref().tile_set.is_some(),
        };
        ui.send_message(WidgetMessage::enabled(
            self.tile_set_button,
            MessageDirection::ToWidget,
            has_tile_set,
        ));
        self.sync_to_state(ui);
    }
    pub fn sync_to_state(&self, ui: &mut UserInterface) {
        fn highlight_all_except(
            button: Handle<UiNode>,
            buttons: &[Handle<UiNode>],
            highlight: bool,
            ui: &UserInterface,
        ) {
            for other_button in buttons {
                if *other_button == button {
                    highlight_tool_button(*other_button, highlight, ui);
                } else {
                    highlight_tool_button(*other_button, !highlight, ui);
                }
            }
        }
        fn highlight_all(buttons: &[Handle<UiNode>], highlight: bool, ui: &UserInterface) {
            for button in buttons {
                highlight_tool_button(*button, highlight, ui);
            }
        }
        let buttons = [
            self.pick_button,
            self.draw_button,
            self.erase_button,
            self.flood_fill_button,
            self.rect_fill_button,
            self.nine_slice_button,
            self.line_button,
        ];
        let state = self.state.lock();
        highlight_tool_button(self.random_button, state.random_mode, ui);
        match state.drawing_mode {
            DrawingMode::Draw => {
                highlight_all_except(self.draw_button, &buttons, true, ui);
            }
            DrawingMode::Erase => {
                highlight_all_except(self.erase_button, &buttons, true, ui);
            }
            DrawingMode::FloodFill => {
                highlight_all_except(self.flood_fill_button, &buttons, true, ui);
            }
            DrawingMode::Pick { .. } => {
                highlight_all_except(self.pick_button, &buttons, true, ui);
            }
            DrawingMode::RectFill { .. } => {
                highlight_all_except(self.rect_fill_button, &buttons, true, ui);
            }
            DrawingMode::NineSlice { .. } => {
                highlight_all_except(self.nine_slice_button, &buttons, true, ui);
            }
            DrawingMode::Line { .. } => {
                highlight_all_except(self.line_button, &buttons, true, ui);
            }
            _ => {
                highlight_all(&buttons, false, ui);
            }
        }
        ui.send_message(PaletteMessage::sync_to_state(
            self.preview,
            MessageDirection::ToWidget,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.pages,
            MessageDirection::ToWidget,
        ));
        ui.send_message(PaletteMessage::sync_to_state(
            self.palette,
            MessageDirection::ToWidget,
        ));
    }
}
