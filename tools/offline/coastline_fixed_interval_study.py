#!/usr/bin/env python3
"""Finite i128/Q interval *arithmetic* screen, with pre-enclosed trig inputs.

No target implementation, full trig evaluator, source curve or device budget.
All intermediates and stored endpoints are checked against signed i128 limits.
"""
import argparse
from fractions import Fraction as F
from math import isqrt
import json
from pathlib import Path

from coastline_interval_predicate_study import I, pi_interval, trig_interval, equatorial_enclosures
from coastline_preflight import A, HEIGHT
from coastline_unknown_precision_study import synthetic_threshold_probes

MIN_I128, MAX_I128 = -(1 << 127), (1 << 127)-1


def checked(value):
    if value < MIN_I128 or value > MAX_I128:
        raise OverflowError('signed i128 intermediate or endpoint exceeded')
    return value


def floor_quotient(a, b):
    if b <= 0:
        raise ValueError('positive denominator required')
    return a//b


def ceil_quotient(a, b):
    q = floor_quotient(a, b)
    return q + (a % b != 0)


class Fixed:
    """Interval of integer endpoints in units of 2^-bits, outward rounded."""
    def __init__(self, lo, hi, bits):
        self.lo, self.hi, self.bits = checked(lo), checked(hi), bits
        if lo > hi or bits < 1 or bits > 126:
            raise ValueError('invalid fixed interval')

    @property
    def scale(self):
        return 1 << self.bits

    @classmethod
    def from_rational(cls, rational, bits):
        interval = rational if isinstance(rational, I) else I(rational)
        scale = 1 << bits
        # Offline rational oracle/precomputed constants have unbounded width;
        # only the quantized integer endpoints enter the checked-width path.
        return cls(checked(floor_quotient(interval.lo.numerator*scale,
                                           interval.lo.denominator)),
                   checked(ceil_quotient(interval.hi.numerator*scale,
                                           interval.hi.denominator)), bits)

    def _operand(self, other):
        if isinstance(other, Fixed):
            if other.bits != self.bits:
                raise ValueError('mixed fixed scales')
            return other
        return Fixed.from_rational(other, self.bits)

    def __add__(self, other):
        other = self._operand(other)
        return Fixed(checked(self.lo+other.lo), checked(self.hi+other.hi), self.bits)

    __radd__ = __add__

    def __neg__(self):
        return Fixed(checked(-self.hi), checked(-self.lo), self.bits)

    def __sub__(self, other):
        return self+-self._operand(other)

    def __rsub__(self, other):
        return self._operand(other)+-self

    def __mul__(self, other):
        other = self._operand(other)
        products = [checked(a*b) for a in (self.lo, self.hi)
                    for b in (other.lo, other.hi)]
        return Fixed(checked(floor_quotient(min(products), self.scale)),
                     checked(ceil_quotient(max(products), self.scale)), self.bits)

    __rmul__ = __mul__

    def __truediv__(self, other):
        other = self._operand(other)
        if other.lo <= 0:
            raise ValueError('nonpositive interval denominator')
        quotients = [(checked(a*self.scale), b) for a in (self.lo, self.hi)
                     for b in (other.lo, other.hi)]
        return Fixed(checked(min(floor_quotient(a, b) for a, b in quotients)),
                     checked(max(ceil_quotient(a, b) for a, b in quotients)), self.bits)

    def sqrt(self):
        if self.lo < 0:
            raise ValueError('negative interval square root')
        a, b = isqrt(checked(self.lo*self.scale)), isqrt(checked(self.hi*self.scale))
        return Fixed(a, checked(b+(checked(b*b) < checked(self.hi*self.scale))), self.bits)

    def sign(self):
        return 'positive' if self.lo > 0 else 'negative' if self.hi < 0 else 'unknown'

    def encloses(self, interval):
        return F(self.lo, self.scale) <= interval.lo and F(self.hi, self.scale) >= interval.hi

    def report(self):
        return {'sign': self.sign(), 'lo_approx': f'{self.lo/self.scale:.9e}',
                'hi_approx': f'{self.hi/self.scale:.9e}',
                'width_approx': f'{(self.hi-self.lo)/self.scale:.6e}',
                'endpoint_max_bits': max(abs(self.lo).bit_length(), abs(self.hi).bit_length())}


def equatorial_fixed(lon, fov, bits):
    """Consume *offline rational* trig enclosures; only algebra is fixed-width."""
    if not 0 <= lon <= 90 or not 0 < fov <= 100:
        raise ValueError('control only covers eastern equator and supported FOV')
    pi = pi_interval()
    sl, cl = trig_interval(F.from_float(lon)*pi/180)
    sf, cf = trig_interval(F.from_float(fov)*pi/360)
    sin_lon, cos_lon, sin_fov, cos_fov = (Fixed.from_rational(x, bits)
                                          for x in (sl, cl, sf, cf))
    a, h, pad = (Fixed.from_rational(F.from_float(x), bits)
                 for x in (A, HEIGHT, .1))
    scale = sin_fov/cos_fov*F(173, 117)
    front = (a+h)*cos_lon-a
    cull = scale*h-scale*a*(cos_lon-1)-a*sin_lon+pad*(1+scale*scale).sqrt()
    return front, cull


def study(output):
    probes = synthetic_threshold_probes()
    report = {'bits': [32, 40, 48, 52, 56],
              'arithmetic': 'signed i128 checked for runtime algebra intermediates/endpoints; offline rational quantization is unrestricted',
              'trigonometry': 'pre-enclosed offline rational inputs, not fixed-point trig',
              'cases': {}}
    for bits in report['bits']:
        cases = {}
        for group, key, fov in (('front_near_tangent', 'input_lon_deg', 20.),
                                ('cull_near_edge', 'lon_deg', .1)):
            cases[group] = []
            for row in probes[group]:
                lon = row[key]
                try:
                    f, c = equatorial_fixed(lon, fov, bits)
                    exact_f, exact_c = equatorial_enclosures(lon, fov)
                    target = f if group == 'front_near_tangent' else c
                    valid = f.encloses(exact_f) and c.encloses(exact_c)
                    if not valid:
                        raise AssertionError('fixed interval did not enclose rational control')
                    result = dict(longitude_deg=lon, **target.report(),
                                  encloses_rational_control=True)
                except OverflowError as error:
                    result = {'longitude_deg': lon, 'overflow': str(error)}
                cases[group].append(result)
        report['cases'][str(bits)] = cases
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
