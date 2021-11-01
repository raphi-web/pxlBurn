
mod tiles;

pub fn split_raster_tile(
    rast: &Vec<Vec<f64>>,
    shape: (usize, usize),
    bounds: (f64, f64, f64, f64),
    resolution: (f64, f64),
) -> Vec<Tile> {
    let (nrows, ncols) = shape;
    let (left, bottom, _right, top) = bounds;

    let (row_half, col_half) = (nrows / 2, ncols / 2);

    let r1 = row_half;
    let r2 = nrows - row_half;

    let c1 = col_half;
    let c2 = ncols - col_half;

    let (h1_bottom, h1_top) = (top + resolution.1 * r1 as f64, top);
    let (h2_bottom, h2_top) = (bottom, bottom - resolution.1 * r2 as f64);

    let h1 = rast[0..row_half].to_vec();
    let h2 = rast[row_half..nrows].to_vec();

    let mut q1: Vec<Vec<f64>> = vec![];
    for row in h1.iter() {
        q1.push(row[0..col_half].to_vec());
    }

    let mut q2: Vec<Vec<f64>> = vec![];
    for row in h1.iter() {
        q2.push(row[col_half..ncols].to_vec());
    }

    let mut q3: Vec<Vec<f64>> = vec![];
    for row in h2.iter() {
        q3.push(row[0..col_half].to_vec());
    }

    let mut q4: Vec<Vec<f64>> = vec![];
    for row in h2.iter() {
        q4.push(row[col_half..ncols].to_vec());
    }

    let quadrents = vec![q1, q2, q3, q4];

    let h1_origin: (usize, usize) = (0, 0);
    let h2_origin: (usize, usize) = (row_half, 0);

    let (q1_left, q1_right) = (left, left + resolution.0 * c1 as f64);
    let (q1_bottom, q1_top) = (h1_bottom, h1_top);

    let (q2_left, q2_right) = (q1_right, q1_right + resolution.0 * c2 as f64);
    let (q2_bottom, q2_top) = (h1_bottom, h1_top);

    let (q3_left, q3_right) = (q1_left, q1_right);
    let (q3_bottom, q3_top) = (h2_bottom, h2_top);

    let (q4_left, q4_right) = (q2_left, q2_right);
    let (q4_bottom, q4_top) = (h2_bottom, h2_top);

    let tile_boundaries = vec![
        (q1_left, q1_bottom, q1_right, q1_top),
        (q2_left, q2_bottom, q2_right, q2_top),
        (q3_left, q3_bottom, q3_right, q3_top),
        (q4_left, q4_bottom, q4_right, q4_top),
    ];

    let q1_origin: (usize, usize) = h1_origin;
    let q2_origin: (usize, usize) = (0, c1);
    let q3_origin: (usize, usize) = (h2_origin.0, 0);
    let q4_origin: (usize, usize) = (h2_origin.0, c1);

    let origins = [q1_origin, q2_origin, q3_origin, q4_origin];

    let mut tile_recs: Vec<geo_types::Polygon<f64>> = Vec::new();
    for b in tile_boundaries.iter() {
        tile_recs.push(mk_rectangle(*b));
    }

    let shapes = [(r1, c1), (r1, c2), (r2, c1), (r2, c2)];
    let mut new_tiles: Vec<Tile> = Vec::new();
    for i in 0..4 {
        let nt = Tile {
            bounds: tile_boundaries[i],
            rectangle: tile_recs[i].clone(),
            origin: origins[i],
            shape: shapes[i],
            parts: TileParts::Raster(quadrents[i].clone()),
            resolution: resolution,
            mother: false,
        };

        new_tiles.push(nt);
    }

    new_tiles
}



pub fn get_coordinates(
    /* Gets the center coordinates of a pixel */
    row: usize,
    col: usize,
    resolution: (f64, f64),
    left: f64,
    top: f64,
) -> (f64, f64) {
    let x = left + (resolution.0 / 2.0) + ((col as f64) * resolution.0);
    let y = top + (resolution.1 / 2.0) + ((row as f64) * resolution.1);
    return (x, y);
}

pub fn mk_rectangle(bounds: (f64, f64, f64, f64)) -> geo::Polygon<f64> {
    /*Helper functon that returns a rectangle */
    let (left, bottom, right, top) = bounds;
    let polygon = geo::Polygon::new(
        geo_types::LineString::from(vec![
            (left, bottom),
            (left, top),
            (right, top),
            (right, bottom),
            (left, bottom),
        ]),
        vec![],
    );
    return polygon;
}