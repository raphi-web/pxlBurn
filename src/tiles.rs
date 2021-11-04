use geo::intersects::Intersects;
extern crate geo_booleanop;

use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use std::thread::{self};
use wkt::ToWkt;
use wkt::{self};
#[derive(Clone)]
pub struct Tile {
    /*
    Tile is a Polygon that covers some part of the raster
    A Tile can have recursive sub tiles which are children of the tile
    When a tile is split it is divided into 4 parts
    */
    pub origin: (usize, usize),
    pub shape: (usize, usize),
    pub bounds: (f64, f64, f64, f64),
    pub resolution: (f64, f64),
    pub rectangle: geo::Polygon<f64>,
    pub parts: Option<Vec<Tile>>,
}

impl Tile {
    pub fn new(
        shape: (usize, usize),
        bounds: (f64, f64, f64, f64),
        origin: (usize, usize),
        resolution: (f64, f64),
    ) -> Self {
        let rows = shape.0;
        let cols = shape.1;

        Self {
            resolution,
            origin,
            bounds,
            shape: (rows, cols),
            rectangle: mk_rectangle(bounds),
            parts: None,
        }
    }

    pub fn split(&mut self, n: usize) {
        /*
        Recursively splits tile into subparts.
        if the tile has children it splits the last 4 children into subtiles
        if the tile has no children it belongs to the last 4 childs
        */
        for _ in 0..n {
            match &mut self.parts {
                Some(tiles) => {
                    for t in tiles.iter_mut() {
                        t.split(1);
                    }
                    self.parts = Some(tiles.clone());
                }
                _ => self.split_tile(),
            }
        }
    }

    fn intersects_tile(&self, geom: &geo::Geometry<f64>) -> Vec<Tile> {
        /*
        Recusively checks if any of  the last children of a tile
        intersect with a Geometry (Polygon, Line etc.).
        Returns vector of  last childs that intersect the geometry
        */

        let mut intersecting_tiles: Vec<Tile> = Vec::new();
        intersects(self, geom, &mut intersecting_tiles);

        fn intersects(tile: &Tile, geom: &geo::Geometry<f64>, intersecting_tiles: &mut Vec<Tile>) {
            if tile.rectangle.intersects(geom) {
                match &tile.parts {
                    Some(tile_vec) => {
                        for t in tile_vec {
                            intersects(t, geom, intersecting_tiles)
                        }
                    }
                    _ => intersecting_tiles.push(tile.clone()),
                }
            }
        }

        intersecting_tiles
    }

    pub fn burn(
        &self,
        geom: Arc<geo::Geometry<f64>>,
        raster: &mut Arc<Mutex<Vec<Vec<f64>>>>,
        burn_value: f64,
        nthreads: usize,
    ) {
        /*
        Burns a given value into a raster, by getting the indexes of the pixel that intersect from
        the  intersectsTile, it then iterates over the Tiles and checks if the remaining pixels
        in the tiles intersect with the geometry

        */

        print!("begin burn..");
        let tile_vec = self.intersects_tile(&geom);
        let ntiles = tile_vec.len();

        let chunck_size = ntiles / nthreads;

        let mut chunks: Vec<Vec<Tile>> = Vec::new();
        if ntiles >= nthreads {
            let mut i = 0;
            loop {
                let stop = if ntiles > i + chunck_size {
                    i + chunck_size
                } else {
                    ntiles
                };
                let chunk = tile_vec[i..stop].to_vec();
                chunks.push(chunk);

                i += chunck_size;
                if i > ntiles {
                    break;
                }
            }
        } else {
            chunks.push(tile_vec);
        }

        let mut handles = vec![];
        for chunk in chunks {
            let bounds = self.bounds;
            let geometry = Arc::clone(&geom);
            let rast = Arc::clone(raster);
            let handle = thread::spawn(move || {
                for t in chunk.iter() {
                    let (nrows, ncols) = t.shape;
                    let (o_row, o_col) = t.origin;
                    let bounds = bounds;
                    let res = t.resolution;

                    for r in o_row..o_row + nrows {
                        for c in o_col..o_col + ncols {
                            let (x, y) = get_coordinates(r, c, res, bounds.0, bounds.3);
                            let left = x - (res.0 / 2.);
                            let right = x + (res.0 / 2.);
                            let bottom = y - (res.1 / 2.);
                            let top = y + (res.1 / 2.);

                            let pxl_poly = mk_rectangle((left, bottom, right, top));

                            if pxl_poly.intersects(&*geometry) {
                                let mut raster = rast.lock().unwrap();
                                raster[r][c] = burn_value;
                            }
                        }
                    }
                }
            });
            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap()
        }
    }

    pub fn export(&self) {
        // for debugging, prints the tiles as well known test to the console
        let n: geo_types::Geometry<f64> =
            geo_types::Geometry::try_from(self.rectangle.clone()).unwrap();
        let n = &n.to_wkt().items[0];

        println!("{}", n);

        match &self.parts {
            Some(tiles) => {
                for t in tiles.iter() {
                    t.export();
                }
            }
            _ => {}
        }
    }

    fn split_tile(&mut self) {
        /*  splits a tile into 4 child tiles and adds these childs
            to tile.parts
        */
        let (nrows, ncols) = self.shape;
        let (left, bottom, _right, top) = self.bounds;

        let (row_half, col_half) = (nrows / 2, ncols / 2);

        let r1 = row_half;
        let r2 = nrows - row_half;

        let c1 = col_half;
        let c2 = ncols - col_half;

        let (h1_bottom, h1_top) = (top + self.resolution.1 * r1 as f64, top);
        let (h2_bottom, h2_top) = (bottom, bottom - self.resolution.1 * r2 as f64);

        let (q1_left, q1_right) = (left, left + self.resolution.0 * c1 as f64);
        let (q1_bottom, q1_top) = (h1_bottom, h1_top);

        let (q2_left, q2_right) = (q1_right, q1_right + self.resolution.0 * c2 as f64);
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

        let (orow, ocol) = self.origin;

        let q1_origin: (usize, usize) = (orow, ocol);
        let q2_origin: (usize, usize) = (orow, ocol + c2);
        let q3_origin: (usize, usize) = (orow + r2, ocol);
        let q4_origin: (usize, usize) = (orow + r2, ocol + c2);

        let origins = [q1_origin, q2_origin, q3_origin, q4_origin];

        let mut tile_recs: Vec<geo_types::Polygon<f64>> = Vec::new();
        for b in tile_boundaries.iter() {
            tile_recs.push(mk_rectangle(*b));
        }

        let shapes = [(r1, c1), (r1, c2), (r2, c1), (r2, c2)];
        let mut new_tiles: Vec<Tile> = Vec::new();
        for i in 0..4 {
            let nt = Tile {
                resolution: self.resolution,
                bounds: tile_boundaries[i],
                rectangle: tile_recs[i].clone(),
                origin: origins[i],
                shape: shapes[i],
                parts: None,
            };
            new_tiles.push(nt);
        }
        self.parts = Some(new_tiles)
    }
}

pub fn mk_rectangle(bounds: (f64, f64, f64, f64)) -> geo::Polygon<f64> {
    /*Helper functon that returns a rectangle */
    let (left, bottom, right, top) = bounds;

    geo::Polygon::new(
        geo_types::LineString::from(vec![
            (left, bottom),
            (left, top),
            (right, top),
            (right, bottom),
            (left, bottom),
        ]),
        vec![],
    )
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
    (x, y)
}
