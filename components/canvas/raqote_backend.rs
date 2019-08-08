/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use crate::canvas_data::{
    Backend, CanvasPaintState, Color, CompositionOp, DrawOptions, ExtendMode, Filter,
    GenericDrawTarget, GenericPathBuilder, GradientStop, GradientStops, Path, Pattern,
    SourceSurface, StrokeOptions, SurfaceFormat,
};
use crate::canvas_paint_thread::AntialiasMode;
use canvas_traits::canvas::*;
use cssparser::RGBA;
use euclid::default::{Point2D, Rect, Size2D, Transform2D, Vector2D};
use std::marker::PhantomData;

pub struct RaqoteBackend;

impl Backend for RaqoteBackend {
    fn get_composition_op(&self, _opts: &DrawOptions) -> CompositionOp {
        unimplemented!()
    }

    fn need_to_draw_shadow(&self, color: &Color) -> bool {
        color.as_raqote().a != 0
    }

    fn size_from_pattern(&self, rect: &Rect<f32>, pattern: &Pattern) -> Option<Size2D<f32>> {
        match pattern {
            Pattern::Raqote(raqote::Source::Image(image, extend, ..)) => match extend {
                raqote::ExtendMode::Repeat => Some(rect.size),
                _ => Some(Size2D::new(image.width as f32, image.height as f32)),
            },
            _ => None,
        }
    }

    fn set_shadow_color<'a>(&mut self, _color: RGBA, _state: &mut CanvasPaintState<'a>) {
        unimplemented!()
    }

    fn set_fill_style<'a>(
        &mut self,
        style: FillOrStrokeStyle,
        state: &mut CanvasPaintState<'a>,
        _drawtarget: &dyn GenericDrawTarget,
    ) {
        if let Some(source) = style.to_raqote_source() {
            state.fill_style = Pattern::Raqote(source);
        }
    }

    fn set_stroke_style<'a>(
        &mut self,
        _style: FillOrStrokeStyle,
        _state: &mut CanvasPaintState<'a>,
        _drawtarget: &dyn GenericDrawTarget,
    ) {
        unimplemented!()
    }

    fn set_global_composition<'a>(
        &mut self,
        _op: CompositionOrBlending,
        _state: &mut CanvasPaintState<'a>,
    ) {
        unimplemented!()
    }

    fn create_drawtarget(&self, size: Size2D<u64>) -> Box<dyn GenericDrawTarget> {
        Box::new(raqote::DrawTarget::new(
            size.width as i32,
            size.height as i32,
        ))
    }

    fn recreate_paint_state<'a>(&self, _state: &CanvasPaintState<'a>) -> CanvasPaintState<'a> {
        CanvasPaintState::new(AntialiasMode::Default)
    }
}

impl<'a> CanvasPaintState<'a> {
    pub fn new(_antialias: AntialiasMode) -> CanvasPaintState<'a> {
        let solid_src = raqote::SolidSource {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        };
        CanvasPaintState {
            draw_options: DrawOptions::Raqote(raqote::DrawOptions::new()),
            fill_style: Pattern::Raqote(raqote::Source::Solid(solid_src)),
            stroke_style: Pattern::Raqote(raqote::Source::Solid(solid_src)),
            stroke_opts: StrokeOptions::Raqote(Default::default(), PhantomData),
            transform: Transform2D::identity(),
            shadow_offset_x: 0.0,
            shadow_offset_y: 0.0,
            shadow_blur: 0.0,
            shadow_color: Color::Raqote(raqote::SolidSource {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            }),
        }
    }
}

impl Pattern<'_> {
    pub fn is_zero_size_gradient(&self) -> bool {
        match self {
            Pattern::Raqote(p) => {
                use raqote::Source::*;

                match p {
                    LinearGradient(g, ..) |
                    RadialGradient(g, ..) |
                    TwoCircleRadialGradient(g, ..) => g.stops.is_empty(),
                    _ => false,
                }
            },
        }
    }
    pub fn as_raqote(&self) -> &raqote::Source {
        match self {
            Pattern::Raqote(p) => p,
        }
    }
}

impl<'a> StrokeOptions<'a> {
    pub fn set_line_width(&mut self, _val: f32) {
        match self {
            StrokeOptions::Raqote(options, _) => options.width = _val,
        }
    }
    pub fn set_miter_limit(&mut self, _val: f32) {
        match self {
            StrokeOptions::Raqote(options, _) => options.miter_limit = _val,
        }
    }
    pub fn set_line_join(&mut self, _val: LineJoinStyle) {
        match self {
            StrokeOptions::Raqote(options, _) => options.join = _val.to_raqote_style(),
        }
    }
    pub fn set_line_cap(&mut self, _val: LineCapStyle) {
        match self {
            StrokeOptions::Raqote(options, _) => options.cap = _val.to_raqote_style(),
        }
    }
    pub fn as_raqote(&self) -> &raqote::StrokeStyle {
        match self {
            StrokeOptions::Raqote(options, _) => options,
        }
    }
}

impl DrawOptions {
    pub fn set_alpha(&mut self, _val: f32) {
        match self {
            DrawOptions::Raqote(draw_options) => draw_options.alpha = _val,
        }
    }
    pub fn as_raqote(&self) -> &raqote::DrawOptions {
        match self {
            DrawOptions::Raqote(options) => options,
        }
    }
}

impl Path {
    pub fn transformed_copy_to_builder(
        &self,
        _transform: &Transform2D<f32>,
    ) -> Box<dyn GenericPathBuilder> {
        unimplemented!()
    }

    pub fn contains_point(&self, _x: f64, _y: f64, _path_transform: &Transform2D<f32>) -> bool {
        unimplemented!()
    }

    pub fn copy_to_builder(&self) -> Box<dyn GenericPathBuilder> {
        unimplemented!()
    }

    pub fn as_raqote(&self) -> &raqote::Path {
        match self {
            Path::Raqote(p) => p,
        }
    }
}

impl GenericDrawTarget for raqote::DrawTarget {
    fn clear_rect(&mut self, rect: &Rect<f32>) {
        let mut pb = raqote::PathBuilder::new();
        pb.rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        );
        raqote::DrawTarget::fill(
            self,
            &pb.finish(),
            &raqote::Source::Solid(raqote::SolidSource {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            }),
            &raqote::DrawOptions::new(),
        );
    }
    fn copy_surface(
        &mut self,
        _surface: SourceSurface,
        _source: Rect<i32>,
        _destination: Point2D<i32>,
    ) {
        unimplemented!();
    }
    fn create_gradient_stops(
        &self,
        _gradient_stops: Vec<GradientStop>,
        _extend_mode: ExtendMode,
    ) -> GradientStops {
        unimplemented!();
    }
    fn create_path_builder(&self) -> Box<dyn GenericPathBuilder> {
        Box::new(PathBuilder::new())
    }
    fn create_similar_draw_target(
        &self,
        _size: &Size2D<i32>,
        _format: SurfaceFormat,
    ) -> Box<dyn GenericDrawTarget> {
        unimplemented!();
    }
    fn create_source_surface_from_data(
        &self,
        _data: &[u8],
        _size: Size2D<i32>,
        _stride: i32,
    ) -> Option<SourceSurface> {
        unimplemented!();
    }
    fn draw_surface(
        &self,
        _surface: SourceSurface,
        _dest: Rect<f64>,
        _source: Rect<f64>,
        _filter: Filter,
        _draw_options: &DrawOptions,
    ) {
        unimplemented!();
    }
    fn draw_surface_with_shadow(
        &self,
        _surface: SourceSurface,
        _dest: &Point2D<f32>,
        _color: &Color,
        _offset: &Vector2D<f32>,
        _sigma: f32,
        _operator: CompositionOp,
    ) {
        unimplemented!();
    }
    fn fill(&mut self, path: &Path, pattern: Pattern, draw_options: &DrawOptions) {
        self.fill(
            path.as_raqote(),
            pattern.as_raqote(),
            draw_options.as_raqote(),
        );
    }
    fn fill_rect(
        &mut self,
        rect: &Rect<f32>,
        pattern: Pattern,
        _draw_options: Option<&DrawOptions>,
    ) {
        let mut pb = raqote::PathBuilder::new();
        pb.rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        );
        raqote::DrawTarget::fill(
            self,
            &pb.finish(),
            pattern.as_raqote(),
            &raqote::DrawOptions::new(),
        );
    }
    fn get_format(&self) -> SurfaceFormat {
        unimplemented!();
    }
    fn get_size(&self) -> Size2D<i32> {
        unimplemented!();
    }
    fn get_transform(&self) -> Transform2D<f32> {
        *self.get_transform()
    }
    fn pop_clip(&mut self) {
        self.pop_clip();
    }
    fn push_clip(&mut self, path: &Path) {
        self.push_clip(path.as_raqote());
    }
    fn set_transform(&mut self, matrix: &Transform2D<f32>) {
        self.set_transform(matrix);
    }
    fn snapshot(&self) -> SourceSurface {
        unimplemented!();
    }
    fn stroke(
        &mut self,
        path: &Path,
        pattern: Pattern,
        stroke_options: &StrokeOptions,
        draw_options: &DrawOptions,
    ) {
        self.stroke(
            path.as_raqote(),
            pattern.as_raqote(),
            stroke_options.as_raqote(),
            draw_options.as_raqote(),
        );
    }
    fn stroke_line(
        &mut self,
        _start: Point2D<f32>,
        _end: Point2D<f32>,
        _pattern: Pattern,
        _stroke_options: &StrokeOptions,
        _draw_options: &DrawOptions,
    ) {
        unimplemented!();
    }
    fn stroke_rect(
        &mut self,
        _rect: &Rect<f32>,
        _pattern: Pattern,
        _stroke_options: &StrokeOptions,
        _draw_options: &DrawOptions,
    ) {
        unimplemented!();
    }
    #[allow(unsafe_code)]
    fn snapshot_data(&self, f: &dyn Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
        let v = self.get_data();
        f(unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<u32>(),
            )
        })
    }
    fn snapshot_data_owned(&self) -> Vec<u8> {
        unimplemented!();
    }
}

struct PathBuilder(Option<raqote::PathBuilder>);

impl PathBuilder {
    fn new() -> PathBuilder {
        PathBuilder(Some(raqote::PathBuilder::new()))
    }
}

impl GenericPathBuilder for PathBuilder {
    fn arc(
        &mut self,
        origin: Point2D<f32>,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        _anticlockwise: bool,
    ) {
        self.0
            .as_mut()
            .unwrap()
            .arc(origin.x, origin.y, radius, start_angle, end_angle);
    }
    fn bezier_curve_to(
        &mut self,
        control_point1: &Point2D<f32>,
        control_point2: &Point2D<f32>,
        control_point3: &Point2D<f32>,
    ) {
        self.0.as_mut().unwrap().cubic_to(
            control_point1.x,
            control_point1.y,
            control_point2.x,
            control_point2.y,
            control_point3.x,
            control_point3.y,
        );
    }
    fn close(&mut self) {
        self.0.as_mut().unwrap().close();
    }
    fn ellipse(
        &mut self,
        _origin: Point2D<f32>,
        _radius_x: f32,
        _radius_y: f32,
        _rotation_angle: f32,
        _start_angle: f32,
        _end_angle: f32,
        _anticlockwise: bool,
    ) {
        unimplemented!();
    }
    fn get_current_point(&mut self) -> Point2D<f32> {
        unimplemented!();
    }
    fn line_to(&mut self, point: Point2D<f32>) {
        self.0.as_mut().unwrap().line_to(point.x, point.y);
    }
    fn move_to(&mut self, point: Point2D<f32>) {
        self.0.as_mut().unwrap().move_to(point.x, point.y);
    }
    fn quadratic_curve_to(&mut self, control_point: &Point2D<f32>, end_point: &Point2D<f32>) {
        self.0.as_mut().unwrap().quad_to(
            control_point.x,
            control_point.y,
            end_point.x,
            end_point.y,
        );
    }
    fn finish(&mut self) -> Path {
        Path::Raqote(self.0.take().unwrap().finish())
    }
}

pub trait ToRaqoteStyle {
    type Target;

    fn to_raqote_style(self) -> Self::Target;
}

impl ToRaqoteStyle for LineJoinStyle {
    type Target = raqote::LineJoin;

    fn to_raqote_style(self) -> raqote::LineJoin {
        match self {
            LineJoinStyle::Round => raqote::LineJoin::Round,
            LineJoinStyle::Bevel => raqote::LineJoin::Bevel,
            LineJoinStyle::Miter => raqote::LineJoin::Miter,
        }
    }
}

impl ToRaqoteStyle for LineCapStyle {
    type Target = raqote::LineCap;

    fn to_raqote_style(self) -> raqote::LineCap {
        match self {
            LineCapStyle::Butt => raqote::LineCap::Butt,
            LineCapStyle::Round => raqote::LineCap::Round,
            LineCapStyle::Square => raqote::LineCap::Square,
        }
    }
}

// TODO(pylbrecht)
#[cfg(feature = "raqote_backend")]
impl Clone for Pattern<'_> {
    fn clone(&self) -> Self {
        unimplemented!();
    }
}

pub trait ToRaqoteSource<'a> {
    fn to_raqote_source(self) -> Option<raqote::Source<'a>>;
}

impl<'a> ToRaqoteSource<'a> for FillOrStrokeStyle {
    fn to_raqote_source(self) -> Option<raqote::Source<'a>> {
        use canvas_traits::canvas::FillOrStrokeStyle::*;

        match self {
            Color(rgba) => Some(raqote::Source::Solid(raqote::SolidSource {
                r: rgba.red,
                g: rgba.green,
                b: rgba.blue,
                a: rgba.alpha,
            })),
            LinearGradient(_) => unimplemented!(),
            RadialGradient(_) => unimplemented!(),
            Surface(_) => unimplemented!(),
        }
    }
}

impl Color {
    fn as_raqote(&self) -> &raqote::SolidSource {
        match self {
            Color::Raqote(s) => s,
        }
    }
}
