use geo::intersects::Intersects;
extern crate geo_booleanop;
use geo_booleanop::boolean::BooleanOp;

use std::convert::TryInto;

use std::thread::{self, JoinHandle};
#[derive(Clone)]
pub struct Tile {
    /*
    A struct representing a tile of a raster.
    One Tile can hold sub_tiles which are also of object tile.
    A Tile can hold raster which is a 2D Vector representing the Pixels
    of the raster

    When splitting one tile into multiple subtiles, the value of raster is emptied and moved to the
    subtiles.

    */
    pub origin: (usize, usize),
    pub shape: (usize, usize),
    pub bounds: (f64, f64, f64, f64),
    pub rectangle: geo_types::Polygon<f64>,
    pub resolution: (f64, f64),
    pub sub_tiles: Vec<Tile>,
    pub raster: Vec<Vec<f64>>,
    pub mother: bool,
}

impl Tile {
    pub fn new(
        raster: &Vec<Vec<f64>>,
        bounds: (f64, f64, f64, f64),
        origin: (usize, usize),
        resolution: (f64, f64),
    ) -> Self {
        let rows = raster.len();
        let cols = raster[0].len();

        let sub_tiles: Vec<Tile> = Vec::new();
        Self {
            origin: origin,
            bounds: bounds,
            shape: (rows, cols),
            rectangle: mk_rectangle(bounds),
            sub_tiles: sub_tiles,
            raster: raster.clone(),
            resolution: resolution,
            mother: true,
        }
    }

    pub fn split(&mut self) {
        /*Splits a Tile into 4 Parts each part is of type tile and corresponds to
        North-East, Nort-West, South-East, South-West part of the tile/raster */
        if self.sub_tiles.len() == 0 {
            let (nrows, ncols) = self.shape;
            let (left, bottom, _right, top) = self.bounds;

            let (row_half, col_half) = (nrows / 2, ncols / 2);

            let r1 = row_half;
            let r2 = nrows - row_half;

            let c1 = col_half;
            let c2 = ncols - col_half;

            let (h1_bottom, h1_top) = (top + self.resolution.1 * r1 as f64, top);
            let (h2_bottom, h2_top) = (bottom, bottom - self.resolution.1 * r2 as f64);

            let h1 = self.raster[0..row_half].to_vec();
            let h2 = self.raster[row_half..nrows].to_vec();

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
                    sub_tiles: Vec::<Tile>::new(),
                    raster: quadrents[i].clone(),
                    resolution: self.resolution,
                    mother: false,
                };

                new_tiles.push(nt);
            }

            self.sub_tiles = new_tiles;
        } else {
            let mut new_tiles: Vec<Tile> = Vec::new();
            for mut t in self.sub_tiles.clone() {
                t.split();
                new_tiles.push(t);
            }
            self.sub_tiles = new_tiles;
            self.raster = Vec::new();
        }
    }

    pub fn recompose(&self) -> (Vec<Vec<f64>>, (usize, usize)) {
        /* Merges all the tiles back to the original Raster */
        let (nrows, ncols) = self.shape;

        if self.raster.len() == 0 {
            let mut new_rast: Vec<Vec<f64>> = vec![vec![0.; ncols]; nrows];
            let mut threads: Vec<JoinHandle<(Vec<Vec<f64>>, (usize, usize))>> = Vec::new();

            if self.mother {
                for p in self.sub_tiles.iter() {
                    let p = p.clone();
                    let t = thread::spawn(move || {
                        let (rast, origin) = p.recompose();
                        (rast, origin)
                    });
                    threads.push(t);
                }

                for t in threads {
                    let (rast, origin) = t.join().unwrap();
                    let (p_rows, p_cols) = (rast.len(), rast[0].len());
                    let (mut ii, mut jj) = (0, 0);
                    for i in origin.0..(origin.0 + p_rows) {
                        for j in origin.1..origin.1 + p_cols {
                            new_rast[i][j] = rast[ii][jj];

                            jj += 1;
                        }
                        ii += 1;
                        jj = 0;
                    }
                }
            } else {
                for p in self.sub_tiles.iter() {
                    let (rast, origin) = p.recompose();
                    let (p_rows, p_cols) = (rast.len(), rast[0].len());

                    let (mut ii, mut jj) = (0, 0);
                    for i in origin.0..(origin.0 + p_rows) {
                        for j in origin.1..origin.1 + p_cols {
                            new_rast[i][j] = rast[ii][jj];

                            jj += 1;
                        }
                        ii += 1;
                        jj = 0;
                    }
                }
            }

            return (new_rast, self.origin);
        } else {
            return (self.raster.clone(), self.origin);
        }
    }

    pub fn split_ntimes(&self, n: usize) -> Tile {
        /* Splits a tile multiple times */
        let mut tile = self.clone();
        for _ in 0..n {
            tile.split();
        }

        return tile;
    }

    pub fn burn_from_vector(&mut self, geom: &geo::Geometry<f64>, burn_value: u8) -> Tile {
        /* Sets the pixelvalue of a tile equal to a given burn value if a geometry object
        like Polygon overlaps the bounds of the raster
        */

        let feature_vector = match geom.clone() {
            geo::Geometry::GeometryCollection(geo::GeometryCollection(v)) => v,
            _ => unreachable!(),
        };

        let mut new_feature_vector: Vec<geo::MultiPolygon<f64>> = vec![];
        for feature in feature_vector.iter() {
            let poly_feature: geo::MultiPolygon<f64> = match feature {
                geo::Geometry::MultiPolygon(v) => v.clone(),
                geo::Geometry::Polygon(v) => {
                    let x = vec![v.clone()];
                    let y: geo::MultiPolygon<f64> = x.try_into().unwrap();
                    y
                }

                _ => unreachable!(),
            };
            let clipped_poly = poly_feature.intersection(&self.rectangle);
            new_feature_vector.push(clipped_poly);
        }

        let geom: geo::Geometry<f64> =
            geo::Geometry::GeometryCollection(geo::GeometryCollection(feature_vector));

        let mut new_tile = self.clone();
        if self.raster.len() == 0 {
            if self.mother {
                let mut threads: Vec<JoinHandle<Tile>> = Vec::new();
                for t in new_tile.sub_tiles.iter() {
                    let geom = geom.clone();
                    let t = t.clone();
                    let thread = thread::spawn(move || {
                        if geom.intersects(&t.rectangle) {
                            let t = t.clone().burn_from_vector(&geom, burn_value);
                            t
                        } else {
                            t.clone()
                        }
                    });
                    threads.push(thread);
                }

                let mut new_sub_tiles: Vec<Tile> = Vec::new();
                for t in threads {
                    let value = t.join().unwrap();
                    new_sub_tiles.push(value);
                }
                new_tile.sub_tiles = new_sub_tiles;
            } else {
                let mut new_sub_tiles: Vec<Tile> = Vec::new();
                for t in new_tile.sub_tiles.iter() {
                    if geom.intersects(&t.rectangle) {
                        let t = t.clone().burn_from_vector(&geom, burn_value);
                        new_sub_tiles.push(t);
                    } else {
                        new_sub_tiles.push(t.clone());
                    }
                }
                new_tile.sub_tiles = new_sub_tiles;
            }

            return new_tile;
        } else {
            new_tile.burn(&geom, burn_value);
            return new_tile;
        }
    }

    fn burn(&mut self, geom: &geo_types::Geometry<f64>, burn_value: u8) {
        /*Helper for burn_to_vector function, iterates over the Pixels of the raster */
        assert!(self.raster.len() != 0);

        let bounds = self.bounds;
        let nrows = self.raster.len();
        let ncols = self.raster[0].len();
        let res = self.resolution;

        for i in 0..nrows {
            for j in 0..ncols {
                let (x, y) = get_coordinates(i, j, res, bounds.0, bounds.3);
                let left = x - (res.0 / 2.);
                let right = x + (res.0 / 2.);
                let bottom = y - (res.1 / 2.);
                let top = y + (res.1 / 2.);

                let pxl_poly = mk_rectangle((left, bottom, right, top));

                if pxl_poly.intersects(geom) {
                    self.raster[i][j] = burn_value as f64;
                }
            }
        }
    }
}

fn get_coordinates(
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

fn mk_rectangle(bounds: (f64, f64, f64, f64)) -> geo::Polygon<f64> {
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
