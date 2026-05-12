use wbvector::feature::{Feature, FieldDef, FieldType, FieldValue, Layer};
use wbvector::geometry::{Coord, Geometry};
use wbvector::topojson;

fn main() -> wbvector::Result<()> {
    let mut layer = Layer::new("roads");
    layer.add_field(FieldDef::new("name", FieldType::Text));

    layer.push(Feature {
        fid: 1,
        geometry: Some(Geometry::line_string(vec![
            Coord::xy(0.0, 0.0),
            Coord::xy(1.0, 0.5),
            Coord::xy(2.0, 1.0),
        ])),
        attributes: vec![FieldValue::Text("main_line".to_string())],
    });

    topojson::write(&layer, "roads.topojson")?;
    let out = topojson::read("roads.topojson")?;

    println!("wrote and read back {} feature(s)", out.len());
    Ok(())
}
