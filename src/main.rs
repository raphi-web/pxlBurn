extern crate geo;
extern crate geo_types;
extern crate geojson;
extern crate indicatif;
extern crate structopt;

use geojson::GeoJson;
use gdal::raster::{Buffer, RasterBand};
use gdal::Dataset;

use std::convert::TryInto;
use std::path::PathBuf;
use std::str::FromStr;
use std::{f64, fs};

use structopt::StructOpt;

mod tiles;

#[derive(StructOpt)]
#[structopt(name = "basic")]
struct Cli {
    #[structopt(parse(from_os_str))]
    json_path: PathBuf,

    #[structopt(parse(from_os_str))]
    raster_path: PathBuf,

    #[structopt(parse(from_os_str))]
    output_raster: PathBuf,

    #[structopt(default_value = "1", long, short = "v")]
    burn_value: u8,

    #[structopt(long, short = "z")]
    set_zero: bool,
}

fn main() {
    // access command line arguments
    let args = Cli::from_args();
    let input_geojson = args.json_path.as_path();
    let input_raster = args.raster_path.as_path();
    let output_raster = args.output_raster.into_os_string().into_string().unwrap();

    let burn_value = args.burn_value;
    let set_zero = args.set_zero;

    // get the data of the input raster
    let raster_dataset = Dataset::open(input_raster).expect("Error opening raster file");
    let transform = raster_dataset.geo_transform().unwrap();
    let projection = raster_dataset.projection();
    let rasterband: RasterBand = raster_dataset
        .rasterband(1)
        .expect("Error: Raster-Band could not be read");
    let cols = rasterband.x_size();
    let rows = rasterband.y_size();

    // upper left & resolution
    let (ul_left, xres, _, ul_top, _, yres) = (
        transform[0],
        transform[1],
        transform[2],
        transform[3],
        transform[4],
        transform[5],
    );
    let bounds = (
        ul_left,
        ul_top + yres * rows as f64,
        ul_left - xres * cols as f64,
        ul_top,
    );

    let size: i64 = (rows * cols) as i64;
    let mut rast_vals: Vec<u32> = vec![0; size as usize];

    let rast = if set_zero {
        // create new raster with shape of input raster
        vec![vec![0.; cols]; rows]
    } else {
        // read raster band
        let mut nrast: Vec<Vec<f64>> = Vec::new();
        let rast_vals = &mut rast_vals[..];

        rasterband
            .read_into_slice(
                (0, 0),
                (cols as usize, rows as usize),
                (cols as usize, rows as usize),
                rast_vals,
                None,
            )
            .expect("Error reading Raster File");

        for r in 0..rows {
            let mut col_vec: Vec<f64> = Vec::new();

            for c in 0..cols {
                col_vec.push(rast_vals[r * cols + c] as f64);
            }

            nrast.push(col_vec);
        }
        nrast
    };

    // read the geojson
    let geojson_str =
        fs::read_to_string(input_geojson).expect("Something went wrong reading the GeoJson");
    let geojson = GeoJson::from_str(&geojson_str).expect("Error: Could not decode GeoJson");
    let geom: geo_types::Geometry<f64> = geojson.try_into().unwrap();

    // tile the raster
    let min_tile_shape = 8;
    let (mut mrows, mut mcols) = (rows.clone(), cols.clone());
    let mut number_of_splits: usize = 0;
    loop {
        mrows /= 4;
        mcols /= 4;
        number_of_splits += 1;

        if mcols <= min_tile_shape {
            break;
        }
        if mrows <= min_tile_shape {
            break;
        }
    }
    // split the raster into tiles
    let tile = tiles::Tile::new(&rast, bounds, (0, 0), (xres, yres));
    let mut tile = tile.split_ntimes(number_of_splits);
    let tile = tile.burn_from_vector(&geom, burn_value);

    let (x, _y) = tile.recompose();
    let mut new_rast_vals: Vec<f64> = Vec::new();
    for row in x.iter() {
        new_rast_vals.append(&mut row.clone());
    }

    let driver = gdal::Driver::get("GTiff").unwrap();

    // create output file
    let mut dataset = driver
        .create_with_band_type::<f64>(&output_raster, cols as isize, rows as isize, 1)
        .expect("Could not create output raster");

    // set the geometry parameters
    dataset
        .set_projection(&projection)
        .expect("Error setting Projection");
    dataset
        .set_geo_transform(&transform)
        .expect("Error setting Geo-Transform");

    // create buffer and write butter to file
    let mut rb = dataset.rasterband(1).unwrap();
    let buff: Buffer<f64> = Buffer {
        size: (cols, rows),
        data: new_rast_vals.to_vec(),
    };

    rb.write((0, 0), (cols, rows), &buff)
        .expect("Error writing new Raster to band");
}

println("Done!")
