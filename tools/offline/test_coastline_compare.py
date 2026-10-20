"""Host-only checks for the disposable comparison geometry, not renderer tests."""
import math
import unittest

from coastline_compare import clip_box, draw, interpolate, pieces, reduce_part, segment_might_enter
from coastline_preflight import setup, surface, trial_bytes
from coastline_refine_40km import decode_trial


class ComparisonTests(unittest.TestCase):
    def test_viewport_segments_not_just_endpoints(self):
        self.assertEqual(clip_box((-2, 0), (2, 0)), ((-1, 0), (1, 0)))
        self.assertIsNone(clip_box((-2, 2), (2, 2)))
        self.assertEqual(clip_box((1, -2), (1, 2)), ((1, -1), (1, 1)))

    def test_closed_part_retains_closure_and_corner(self):
        loop = [(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)]
        reduced = reduce_part(loop, 40)
        self.assertEqual(reduced[0], reduced[-1])
        self.assertGreaterEqual(len(reduced), 4)

    def test_hidden_endpoints_surface_crossing_and_tangent_are_bounded(self):
        # Endpoints are hidden behind Earth at an equatorial observer; the
        # *surface curve* passes in front. Never connect the far-side chord.
        layer, stats = draw([[(-89, 0), (89, 0)]], (0, 0, 0), 5)
        self.assertEqual(stats['tangents'], 2)
        self.assertGreater(stats['coast_ink'], 0)
        self.assertGreater(stats['surface_intervals'], 0)
        for y in range(240):
            for x in range(400):
                if layer.getpixel((x, y)) == 0:
                    self.assertTrue(3 <= x <= 349 and 3 <= y <= 237)

    def test_shortest_antimeridian_path_is_not_global_chord(self):
        layer, stats = draw([[(179.9, 0), (-179.9, 0)]], (0, 179.9, 0), 5)
        self.assertEqual(stats['surface_intervals'], 1)
        self.assertLess(stats['coast_ink'], 20)
        self.assertGreater(stats['coast_ink'], 0)

    def test_sphere_cull_keeps_two_offscreen_endpoint_crossing(self):
        site = (0, 0, 0)
        observer, _, east, north, up = setup(site)
        self.assertTrue(segment_might_enter((-10, 0), (10, 0), observer, east, north, up,
                                            math.tan(math.radians(5/2))))
        self.assertFalse(segment_might_enter((30, 0), (30.2, 0), observer, east, north, up,
                                             math.tan(math.radians(.1/2))))
        both = [[(-10, 0), (10, 0)]]
        full, _ = draw(both, site, 5)
        culled, stats = draw(both, site, 5, cull=True)
        self.assertEqual(full.tobytes(), culled.tobytes())
        self.assertEqual(stats['coarse_kept'], 1)

    def test_surface_curve_within_geographic_speed_sphere(self):
        # Spot-check the 6500km/rad bound at equator, near pole and dateline.
        for a, b in [((-10, 0), (10, 0)), ((150, 80), (-175, 89.9)),
                     ((179.9, 0), (-179.9, 0)), ((0, -80), (5, 80))]:
            dl = b[0] - a[0]
            if abs(dl) > 180:
                dl -= math.copysign(360, dl)
            bound = 6500 * math.radians(math.hypot(dl, b[1] - a[1]))
            x, _ = surface(*a)
            for i in range(101):
                y, _ = surface(*interpolate(a, b, i / 100))
                self.assertLessEqual(math.dist(x, y), bound + 1e-8)

    def test_trial_encoding_decodes_parts_without_joining(self):
        parts = [[(179.9, 0), (-179.9, 0)], [(0, 80), (1, 81)]]
        packed, _ = trial_bytes(parts)
        decoded = decode_trial(packed, len(parts))
        self.assertEqual(decoded, parts)
        with self.assertRaises(ValueError):
            decode_trial(packed[:-1], len(parts))

    def test_finer_sampling_preserves_simple_edge_and_tangent_masks(self):
        self.assertEqual(len(list(pieces([[(0, 0), (1, 0)]], sample_step=.05))), 20)
        with self.assertRaises(ValueError):
            list(pieces([[(0, 0), (1, 0)]], sample_step=0))
        for part, site, fov in [([[(-2, 0), (2, 0)]], (0, 0, 0), .1),
                                ([[(84.8, .02), (86.2, .02)]], (0, 0, 0), 20)]:
            rough, a = draw(part, site, fov, cull=True, sample_step=.2)
            fine, b = draw(part, site, fov, cull=True, sample_step=.01)
            self.assertEqual(rough.tobytes(), fine.tobytes())
            self.assertEqual(a['tangents'], b['tangents'])


if __name__ == '__main__':
    unittest.main()
