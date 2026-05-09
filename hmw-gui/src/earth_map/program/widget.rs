use iced::advanced::{Layout, Widget, layout, mouse, renderer, widget::Tree};
use iced::widget::{self, shader};
use iced::{Element, Length, Rectangle, Renderer, Size, Theme};

use super::{
    EarthMapProgram, EarthMapProgramMessage, primitive::EarthMapPrimitive, state::EarthMapState,
    types::EarthMapColors,
};

/// A theme-aware shader widget for the interactive earth map.
pub struct EarthMapWidget<'a> {
    width: Length,
    height: Length,
    program: &'a EarthMapProgram,
}

impl<'a> EarthMapWidget<'a> {
    /// Creates a new earth map widget backed by the given shader program.
    pub fn new(program: &'a EarthMapProgram) -> Self {
        Self {
            width: Length::Fixed(100.0),
            height: Length::Fixed(100.0),
            program,
        }
    }

    /// Sets the widget width.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the widget height.
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    fn shader(
        &self,
        colors: EarthMapColors,
    ) -> Element<'_, EarthMapProgramMessage, Theme, Renderer> {
        widget::shader(ThemedProgram {
            program: self.program,
            colors,
        })
        .width(self.width)
        .height(self.height)
        .into()
    }
}

impl Widget<EarthMapProgramMessage, Theme, Renderer> for EarthMapWidget<'_> {
    fn size(&self) -> Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let mut shader = self.shader(EarthMapColors::from_theme(&Theme::Light));
        let child = shader.as_widget_mut().layout(
            tree.children
                .first_mut()
                .expect("earth map shader tree exists"),
            renderer,
            limits,
        );

        layout::Node::with_children(child.size(), vec![child])
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let shader = self.shader(EarthMapColors::from_theme(theme));
        shader.as_widget().draw(
            tree.children.first().expect("earth map shader tree exists"),
            renderer,
            theme,
            style,
            layout.child(0),
            cursor,
            viewport,
        );
    }

    fn children(&self) -> Vec<Tree> {
        let shader = self.shader(EarthMapColors::from_theme(&Theme::Light));

        vec![Tree::new(&shader)]
    }

    fn diff(&self, tree: &mut Tree) {
        let shader = self.shader(EarthMapColors::from_theme(&Theme::Light));

        tree.diff_children(&[shader.as_widget()]);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, EarthMapProgramMessage>,
        viewport: &Rectangle,
    ) {
        let mut shader = self.shader(EarthMapColors::from_theme(&Theme::Light));

        shader.as_widget_mut().update(
            tree.children
                .first_mut()
                .expect("earth map shader tree exists"),
            event,
            layout.child(0),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let shader = self.shader(EarthMapColors::from_theme(&Theme::Light));

        shader.as_widget().mouse_interaction(
            tree.children.first().expect("earth map shader tree exists"),
            layout.child(0),
            cursor,
            viewport,
            renderer,
        )
    }
}

impl<'a> From<EarthMapWidget<'a>> for Element<'a, EarthMapProgramMessage, Theme, Renderer> {
    fn from(widget: EarthMapWidget<'a>) -> Self {
        Element::new(widget)
    }
}

struct ThemedProgram<'a> {
    program: &'a EarthMapProgram,
    colors: EarthMapColors,
}

impl shader::Program<EarthMapProgramMessage> for ThemedProgram<'_> {
    type State = EarthMapState;

    type Primitive = EarthMapPrimitive;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<shader::Action<EarthMapProgramMessage>> {
        self.program.update(state, event, bounds, cursor)
    }

    fn draw(
        &self,
        state: &Self::State,
        cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        self.program.draw(state, cursor, bounds, self.colors)
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        self.program.mouse_interaction(state, bounds, cursor)
    }
}
