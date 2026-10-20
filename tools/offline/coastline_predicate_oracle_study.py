#!/usr/bin/env python3
"""Independent high-precision *narrow equatorial* predicate controls.

Not an interval proof for libm, all observers, curve bounds, or target execution.
Decimal inputs are exact values of the host binary64 inputs and constants.
"""
import argparse
from decimal import Decimal, localcontext
import json
import math
from pathlib import Path

from coastline_compare import segment_might_enter
from coastline_preflight import A, HEIGHT, setup
from coastline_unknown_policy_study import viewport_piece
from coastline_visibility_study import front_value


def d(value):
    return Decimal.from_float(value)


def pi_machin():
    """80-digit π via alternating arctan series; 100+ tail terms."""
    def atan_inverse(n, terms):
        x = Decimal(1)/n
        return sum(((-1)**i*x**(2*i+1)/Decimal(2*i+1)
                    for i in range(terms)), Decimal(0))
    return 16*atan_inverse(5, 100)-4*atan_inverse(239, 30)


def trig(x):
    """Taylor at |x|<π. Only diagnostic high precision, not interval arithmetic."""
    s, c = x, Decimal(1)
    st, ct = x, Decimal(1)
    for n in range(1, 80):
        st *= -x*x/Decimal((2*n)*(2*n+1))
        ct *= -x*x/Decimal((2*n-1)*(2*n))
        s += st
        c += ct
    return s, c


def equatorial_reference(lon, fov, digits):
    """Analytic exact-real equatorial sign/positive-east cull at a=b."""
    with localcontext() as context:
        context.prec = digits
        angle = d(lon)*pi_machin()/180
        sin_lon, cos_lon = trig(angle)
        half_angle = d(fov)*pi_machin()/360
        sin_half, cos_half = trig(half_angle)
        scale = sin_half/cos_half*Decimal(173)/Decimal(117)
        a, height = d(A), d(HEIGHT)
        front = (a+height)*cos_lon-a
        # Frustum plane for east-facing positive-x side at observer lon=0:
        # radius=0.1 km for this zero-length source segment.
        plane = scale*height - scale*a*(cos_lon-1) - a*sin_lon
        cull_margin = plane + d(.1)*(1+scale*scale).sqrt()
        return front, cull_margin


def signed_comparison(lon, fov, digits=80):
    front, cull = equatorial_reference(lon, fov, digits)
    _, camera, *_ = setup((0., 0., 0.))
    host_front = front_value((lon, 0.), camera)
    host_observer, _, east, north, up = setup((0., 0., 0.))
    host_trace = {}
    kept = segment_might_enter((lon, 0.), (lon, 0.), host_observer, east,
                               north, up, math.tan(math.radians(fov/2)),
                               trace=host_trace)
    return {'longitude_deg': lon, 'reference_front_km': str(front),
            'host_front_km': host_front,
            'front_sign_disagrees': (front > 0) != (host_front > 0),
            'reference_positive_east_plane_km': str(cull),
            'host_kept': kept, 'positive_east_cull_disagrees': kept != (cull >= 0),
            'host_closest_plane_km': host_trace['closest_cull_plane_km']}


def study(output):
    site = (0., 0., 0.)
    observer, camera, east, north, up = setup(site)
    tangent = math.degrees(math.acos(A/(A+HEIGHT)))
    front_cases = []
    for direction in (-math.inf, math.inf):
        x = tangent
        for _ in range(16):
            x = math.nextafter(x, direction)
        front_cases.append(signed_comparison(x, 20.))
    span = math.tan(math.radians(.1/2))
    lo, hi = 0., 2.
    for _ in range(60):
        mid = (lo+hi)/2
        if segment_might_enter((mid, 0.), (mid, 0.), observer, east, north, up, span):
            lo = mid
        else:
            hi = mid
    cull_cases = [signed_comparison(x, .1) for x in
                  (math.nextafter(lo, -math.inf), lo, hi,
                   math.nextafter(hi, math.inf))]
    viewport = []
    x = 1+.05/173
    for value in (math.nextafter(x, -math.inf), x, math.nextafter(x, math.inf)):
        disposition, _ = viewport_piece((value, 0.), (value, .1), .05)
        with localcontext() as context:
            context.prec = 80
            # Exact-real test of the expanded chord bounding box from the
            # supplied *binary64* coordinates/error, NOT a WGS-84 projection.
            reference_margin = Decimal(1)-d(value)+d(.05)/173
        viewport.append({'normalized_x': value, 'host_disposition': disposition,
                         'reference_edge_margin_normalized': str(reference_margin),
                         'host_matches_reference': (disposition == 'outside') ==
                                                   (reference_margin < 0)})
    report = {'decimal_precision_digits': 80,
              'inputs': 'exact host-f64 values; equatorial zero-length surface segment',
              'front_near_tangent': front_cases,
              'cull_near_east_edge': cull_cases,
              'viewport_supplied_chord_edge': viewport}
    output.mkdir(parents=True, exist_ok=True)
    (output/'report.json').write_text(json.dumps(report, indent=2)+'\n')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    print(json.dumps(study(args.output), indent=2))


if __name__ == '__main__':
    main()
