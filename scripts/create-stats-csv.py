#!/usr/bin/env python3

import sys
import json
import pandas as P
import argparse
import re
from pyproj import Transformer

GPS_EXIFS = ['GPSLatitude', 'GPSLongitude']
GPS_RE = re.compile('[^0-9.]+')

def get_args():
    parser = argparse.ArgumentParser(description='Vimana VM Command tool')

    parser.add_argument('-g', '--gps',
                        help='Add GPS coords',
                        action='store_true')
    parser.add_argument('--epsg', help='Convert lat long to projected coords (requires --gps)', type=int)
    return parser.parse_args()

def normalize_stats(st: dict):
    count = st['count']
    mean = st['sum'] / count
    var = st['sum_2'] / count - mean * mean
    std = var ** 0.5
    st.update(mean=mean, std_dev=std)
    return st

def degree_to_float(deg_str):
    deg_comps = GPS_RE.split(deg_str)
    deg_float = float(deg_comps[0]) + float(deg_comps[1]) / 60. + float(deg_comps[2]) / 3600.
    if deg_str[-1] == "S" or deg_str[-1] == "W": deg_float = -deg_float
    return deg_float

def add_gps_coords(stat, proj_t: Transformer = None):
    exif = json.load(open(stat['path']))
    if isinstance(exif, list): exif = exif[0]
    lat = degree_to_float(exif['GPSLatitude'])
    lon = degree_to_float(exif['GPSLongitude'])
    stat['latitude'] = lat
    stat['longitude'] = lon
    if proj_t is not None:
        (east, north) = proj_t.transform(lat, lon)
        stat['easting'] = east
        stat['northing'] = north


def main():
    args = get_args()
    stats_json = json.load(sys.stdin)

    proj_t = None
    if args.gps and args.epsg:
        proj_t = Transformer.from_crs(4326, args.epsg)

    df = []
    for im_stat in stats_json['image_stats']:
        stat = normalize_stats(im_stat['stats'])
        im_stat.update(**stat)
        if args.gps: add_gps_coords(im_stat, proj_t=proj_t)
        df.append(im_stat)

    tot = normalize_stats(stats_json['cumulative'])
    tot.update(path="cumulative")
    df.append(tot)

    df = P.DataFrame(df)
    cols = 'path,width,height,min,max,mean,std_dev'.split(',')
    if args.gps:
        cols = cols + ['latitude', 'longitude']
        if args.epsg is not None:
            cols = cols + ['easting', 'northing']
    df = df[cols]
    df.to_csv(sys.stdout, index=False)

if __name__ == '__main__':
    main()
