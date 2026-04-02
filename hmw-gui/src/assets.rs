use std::borrow::Cow;

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "assets"]
pub struct Assets;

impl Assets {
    /// Returns embedded application fonts.
    pub fn fonts() -> Vec<Cow<'static, [u8]>> {
        let font = Self::get("icons.ttf").expect("font exists");
        vec![font.data]
    }

    /// Returns the embedded application logo SVG.
    pub fn logo_svg() -> Cow<'static, [u8]> {
        let svg = Self::get("logo.svg").expect("logo svg exists");
        svg.data
    }

    /// Returns the embedded application logo PNG.
    pub fn logo_png() -> Cow<'static, [u8]> {
        let png = Self::get("logo.png").expect("logo png exists");
        png.data
    }

    /// Returns the embedded earth texture.
    pub fn earth_map_texture() -> Cow<'static, [u8]> {
        let texture = Self::get("earth_map.ktx2").expect("texture exists");
        texture.data
    }

    /// Returns the embedded lattice hash data.
    pub fn lattice_hashes() -> Cow<'static, [u8]> {
        let hashes = Self::get("lattice.json").expect("hashes exist");
        hashes.data
    }

    /// Returns the help icon for selecting a map region.
    pub fn earth_map_help_select_svg() -> Cow<'static, [u8]> {
        let svg = Self::get("earth_map_help_select.svg").expect("svg exists");
        svg.data
    }

    /// Returns the help icon for deselecting a map region.
    pub fn earth_map_help_deselect_svg() -> Cow<'static, [u8]> {
        let svg = Self::get("earth_map_help_deselect.svg").expect("svg exists");
        svg.data
    }

    /// Returns the help icon for map zoom controls.
    pub fn earth_map_help_zoom_svg() -> Cow<'static, [u8]> {
        let svg = Self::get("earth_map_help_zoom.svg").expect("svg exists");
        svg.data
    }

    /// Returns the help icon for globe rotation controls.
    pub fn earth_map_help_rotate_svg() -> Cow<'static, [u8]> {
        let svg = Self::get("earth_map_help_rotate.svg").expect("svg exists");
        svg.data
    }
}
