#!/usr/bin/env python3
"""Rational interval *equatorial point-predicate* feasibility control.

Exact fractions enclose π, sine/cosine and square root. No WGS-84 generalization,
derivative bound, projected curve, target-float implementation or work budget.
"""
import argparse
from fractions import Fraction as F
from math import factorial, isqrt
import json
from pathlib import Path

from coastline_preflight import A, HEIGHT
from coastline_unknown_precision_study import synthetic_threshold_probes


class I:
    def __init__(self, lo, hi=None):
        self.lo = F(lo)
        self.hi = F(lo if hi is None else hi)
        if self.lo > self.hi:
            raise ValueError('invalid interval')

    def __add__(self, other):
        other = as_i(other)
        return I(self.lo+other.lo, self.hi+other.hi)

    __radd__ = __add__

    def __neg__(self):
        return I(-self.hi, -self.lo)

    def __sub__(self, other):
        return self+-as_i(other)

    def __rsub__(self, other):
        return as_i(other)+-self

    def __mul__(self, other):
        other = as_i(other)
        products = (self.lo*other.lo, self.lo*other.hi,
                    self.hi*other.lo, self.hi*other.hi)
        return I(min(products), max(products))

    __rmul__ = __mul__

    def __truediv__(self, other):
        other = as_i(other)
        if other.lo <= 0 <= other.hi:
            raise ValueError('interval division by zero')
        return self*I(1/other.hi, 1/other.lo)

    def sign(self):
        return 'positive' if self.lo > 0 else 'negative' if self.hi < 0 else 'unknown'

    def decimal(self, digits=25):
        return {'lo_approx': f'{float(self.lo):.{digits-1}e}',
                'hi_approx': f'{float(self.hi):.{digits-1}e}',
                'width_approx': f'{float(self.hi-self.lo):.5e}'}

    def max_bits(self):
        return max(self.lo.numerator.bit_length(), self.lo.denominator.bit_length(),
                   self.hi.numerator.bit_length(), self.hi.denominator.bit_length())


def as_i(value):
    return value if isinstance(value, I) else I(value)


def atan_inverse(n, terms):
    total = sum((F((-1)**k, (2*k+1)*n**(2*k+1)) for k in range(terms)), F(0))
    tail = F((-1)**terms, (2*terms+1)*n**(2*terms+1))
    return I(min(total, total+tail), max(total, total+tail))


def pi_interval():
    return 16*atan_inverse(5, 80)-4*atan_inverse(239, 30)


def trig_interval(x, terms=24):
    """Taylor at midpoint, Lagrange error ≤ M^49/49!, plus angle radius.

    Both polynomials run through degree 48 (sin's even coefficient is zero).
    |sin'|,|cos'| ≤ 1 supplies the angle-input widening.
    """
    if x.lo < 0 or x.hi > F(11, 7):
        raise ValueError('control only covers angles in [0, 11/7] radians')
    mid = (x.lo+x.hi)/2
    rad = (x.hi-x.lo)/2
    maximum = max(abs(x.lo), abs(x.hi))
    sin_poly = sum(((-1)**k*mid**(2*k+1)/factorial(2*k+1)
                    for k in range(terms)), F(0))
    cos_poly = sum(((-1)**k*mid**(2*k)/factorial(2*k)
                    for k in range(terms+1)), F(0))
    error = maximum**(2*terms+1)/factorial(2*terms+1)+rad
    return I(sin_poly-error, sin_poly+error), I(cos_poly-error, cos_poly+error)


def sqrt_interval(x, places=70):
    if x.lo < 0:
        raise ValueError('negative square root')
    scale = 10**places
    def bracket(v):
        k = isqrt(v.numerator*scale**2//v.denominator)
        return F(k, scale), F(k+1, scale)
    low, _ = bracket(x.lo)
    _, high = bracket(x.hi)
    return I(low, high)


def equatorial_enclosures(lon, fov):
    """Point at zero latitude/altitude, observer at zero longitude/altitude.

    Input float bit patterns are interpreted as exact rationals; these are
    real-function enclosures, not f64 computations with known errors.
    """
    if not 0 <= lon <= 90 or not 0 < fov <= 100:
        raise ValueError('control only covers eastern equator and supported FOV')
    pi = pi_interval()
    angle = F.from_float(lon)*pi/180
    sin_lon, cos_lon = trig_interval(angle)
    half_angle = F.from_float(fov)*pi/360
    sin_half, cos_half = trig_interval(half_angle)
    scale = sin_half/cos_half*F(173, 117)
    a, h = F.from_float(A), F.from_float(HEIGHT)
    front = (a+h)*cos_lon-a
    plane = scale*h-scale*a*(cos_lon-1)-a*sin_lon
    cull = plane+F.from_float(.1)*sqrt_interval(1+scale*scale)
    return front, cull


def study(output):
    probes = synthetic_threshold_probes()
    reference = {'front_near_tangent': [], 'cull_near_east_edge': [],
                 'viewport_supplied_chord_edge': []}
    for group, key, fov in (('front_near_tangent', 'input_lon_deg', 20.),
                            ('cull_near_east_edge', 'lon_deg', .1)):
        for input_case in probes['cull_near_edge' if group == 'cull_near_east_edge' else group]:
            front, cull = equatorial_enclosures(input_case[key], fov)
            chosen = front if group == 'front_near_tangent' else cull
            host_agrees = ((chosen.sign() == ('positive' if input_case['front_km'] > 0 else 'negative'))
                           if group == 'front_near_tangent' else
                           (chosen.sign() == ('positive' if input_case['kept'] else 'negative')))
            reference[group].append({'longitude_deg': input_case[key],
                                     'enclosed_sign': chosen.sign(), 'host_agrees': host_agrees,
                                     'enclosure_km': chosen.decimal(),
                                     'largest_fraction_bits': chosen.max_bits()})
    for input_case in probes['viewport_near_edge']:
        margin = I(1)-F.from_float(input_case['normalized_x'])+F.from_float(.05)/173
        reference['viewport_supplied_chord_edge'].append({
            'normalized_x': input_case['normalized_x'], 'enclosed_sign': margin.sign(),
            'host_agrees': (input_case['disposition'] == 'outside') ==
                           (margin.sign() == 'negative'),
            'enclosure_normalized': margin.decimal(),
            'largest_fraction_bits': margin.max_bits()})
    report = {'pi_terms_inverse_5': 80, 'pi_terms_inverse_239': 30,
              'sin_terms': 24, 'cos_terms': 25, 'sqrt_decimal_places': 70,
              'inputs': 'exact binary64 values; equatorial point and supplied chord only',
              'cases': reference}
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
