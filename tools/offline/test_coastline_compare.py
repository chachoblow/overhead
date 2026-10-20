"""Host-only checks for the disposable comparison geometry, not renderer tests."""
import math
import unittest

from coastline_compare import clip_box, draw, interpolate, pieces, reduce_part, segment_might_enter
from coastline_preflight import HEIGHT, SITES, dot, setup, surface, trial_bytes
from coastline_refine_40km import decode_trial
from coastline_stroke_study import BOUNDS, difference, ink, stroke, scene
from coastline_precision_study import check_shortest_longitude_branches, vertex_screen_bound
from coastline_visibility_study import (enclose, front_lipschitz, front_second_bound,
                                         front_slope, front_value)
from coastline_curvature_study import chord_bound_px, project
from coastline_enclosure_work_study import sweep
from coastline_unknown_policy_study import illustrative_fallback, trial, viewport_piece
from coastline_unknown_precision_study import compare, synthetic_threshold_probes
from coastline_fallback_appearance_study import cases as fallback_cases
from coastline_atomic_fallback_study import atomic_case
from coastline_predicate_oracle_study import equatorial_reference, signed_comparison
from coastline_interval_predicate_study import (I, equatorial_enclosures,
                                                 pi_interval, sqrt_interval, trig_interval)
from coastline_fixed_interval_study import Fixed, MAX_I128, equatorial_fixed


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

    def test_checked_fixed_interval_feasibility(self):
        from fractions import Fraction
        signed = Fixed.from_rational(Fraction(-1, 3), 2)
        self.assertEqual((signed.lo, signed.hi), (-2, -1))
        self.assertEqual((signed*2).lo, -4)
        self.assertEqual((signed/2).hi, 0)
        with self.assertRaises(OverflowError):
            _ = Fixed(MAX_I128, MAX_I128, 52)*2
        with self.assertRaises(ValueError):
            _ = signed/Fixed(-1, 1, 2)
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        left, right = tangent, tangent
        for _ in range(16):
            left = math.nextafter(left, -math.inf)
            right = math.nextafter(right, math.inf)
        self.assertEqual(equatorial_fixed(left, 20., 40)[0].sign(), 'unknown')
        self.assertEqual(equatorial_fixed(left, 20., 48)[0].sign(), 'positive')
        self.assertEqual(equatorial_fixed(right, 20., 48)[0].sign(), 'negative')
        self.assertEqual(equatorial_fixed(.8702965507260555, .1, 52)[1].sign(),
                         'unknown')
        with self.assertRaises(OverflowError):
            equatorial_fixed(.8702965507260555, .1, 56)

    def test_rational_interval_equatorial_predicates(self):
        from decimal import Decimal
        from fractions import Fraction
        pi = pi_interval()
        self.assertGreater(pi.lo, Fraction('3.14159265358979323846264338327950288419716939937510'))
        self.assertLess(pi.hi, Fraction('3.14159265358979323846264338327950288419716939937511'))
        sine, cosine = trig_interval(I(0))
        self.assertTrue(sine.lo <= 0 <= sine.hi)
        self.assertTrue(cosine.lo <= 1 <= cosine.hi)
        root = sqrt_interval(I(2))
        self.assertLessEqual(root.lo**2, 2)
        self.assertGreaterEqual(root.hi**2, 2)
        self.assertEqual(I(-1, 1).sign(), 'unknown')
        with self.assertRaises(ValueError):
            _ = I(1)/I(-1, 1)
        with self.assertRaises(ValueError):
            trig_interval(I(-1))
        near = .8702965507260555
        _, cull = equatorial_enclosures(near, .1)
        _, exact_cull = equatorial_reference(near, .1, 80)
        self.assertTrue(cull.lo <= Fraction(exact_cull) <= cull.hi)
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        for _ in range(16):
            tangent = math.nextafter(tangent, -math.inf)
        front, _ = equatorial_enclosures(tangent, 20.)
        exact_front, _ = equatorial_reference(tangent, 20., 80)
        self.assertTrue(front.lo <= Fraction(exact_front) <= front.hi)
        self.assertEqual(cull.sign(), 'positive')
        self.assertGreater(cull.max_bits(), 100000)

    def test_high_precision_equatorial_predicate_controls(self):
        from decimal import Decimal
        front, cull = equatorial_reference(0., 20., 80)
        self.assertEqual(front, Decimal.from_float(HEIGHT))
        self.assertGreater(cull, 0)
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        before = tangent
        after = tangent
        for _ in range(16):
            before = math.nextafter(before, -math.inf)
            after = math.nextafter(after, math.inf)
        a = signed_comparison(before, 20.)
        b = signed_comparison(after, 20.)
        self.assertGreater(Decimal(a['reference_front_km']), 0)
        self.assertLess(Decimal(b['reference_front_km']), 0)
        self.assertFalse(a['front_sign_disagrees'] or b['front_sign_disagrees'])
        for lon in (before, after):
            x, _ = equatorial_reference(lon, 20., 80)
            y, _ = equatorial_reference(lon, 20., 95)
            self.assertLess(abs(x-y), Decimal('1e-55'))

    def test_atomic_two_pass_clean_and_forced_budget(self):
        site = (0., 0., 0.)
        clean = [[(-2., 0.), (2., 0.)]]
        a = atomic_case(clean, site, .1)
        self.assertFalse(a['fallback'])
        self.assertEqual(a['preflight_draw_calls'], 0)
        self.assertEqual(a['displayed_ink'], 347)
        self.assertEqual(a['display_vs_local_gap_pixels'], 0)
        self.assertEqual(a['commit_candidate_visits'], a['commit']['pixel_candidates'])
        for key in ('nodes', 'front_subintervals', 'pixel_candidates'):
            self.assertEqual(a['combined_two_pass_proxy'][key], 2 * a['preflight'][key])

        parts, site, fov, cap = fallback_cases([])['mixed-forced-budget']
        b = atomic_case(parts, site, fov, max_nodes=cap)
        self.assertTrue(b['fallback'])
        self.assertIsNone(b['commit'])
        self.assertEqual(b['preflight_draw_calls'], 0)
        self.assertEqual(b['commit_candidate_visits'], 0)
        self.assertEqual(b['displayed_ink'], 0)
        self.assertEqual(b['local_gap_oracle_ink'], 5)
        self.assertEqual(b['display_vs_local_gap_pixels'], 5)
        self.assertEqual(b['combined_two_pass_proxy']['nodes'], b['preflight']['nodes'])

    def test_bounded_streaming_leaves_match_list_oracle_and_keep_unknown(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        site = (0., 0., 0.)
        a, b = (tangent-.3, 0.), (tangent+.3, 0.)
        baseline = enclose(a, b, site, 20., order=2, collect_front=True)
        front, unknown = [], []
        streamed = enclose(a, b, site, 20., order=2, pending_limit=32,
                           collect_unknown=False,
                           on_front=lambda x, y: front.append([x, y]),
                           on_unknown=lambda x, y, budget: unknown.append(([x, y], budget)))
        self.assertEqual(streamed['pending_cap_hits'], 0)
        self.assertLessEqual(streamed['peak_pending'], 32)
        self.assertEqual(front, baseline['front_intervals'])
        self.assertEqual([arc for arc, _ in unknown], baseline['unknown_intervals'])
        self.assertFalse(any(budget for _, budget in unknown))
        for key in baseline.keys() - {'front_intervals', 'unknown_intervals'}:
            self.assertEqual(streamed[key], baseline[key])

        limited_front, limited_unknown = [], []
        limited = enclose(a, b, site, 20., order=2, pending_limit=4,
                          collect_unknown=False,
                          on_front=lambda x, y: limited_front.append((x, y)),
                          on_unknown=lambda x, y, budget:
                          limited_unknown.append((x, y, budget)))
        self.assertEqual(limited['pending_cap_hits'], 1)
        self.assertEqual(limited['peak_pending'], 4)
        self.assertGreater(len(limited_front), 0)  # prior certified work survives
        self.assertEqual(len(limited_unknown), limited['unknown'])
        self.assertEqual(sum(item[2] for item in limited_unknown), limited['budget_unknown'])
        self.assertGreater(limited['budget_unknown'], 0)
        for invalid in (0, -1, 1.5, True):
            with self.assertRaises(ValueError):
                enclose(a, b, site, 20., order=2, pending_limit=invalid)

    def test_two_pass_pending_exhaustion_never_commits_earlier_ink(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        parts = [[(-2., 0.), (2., 0.)], [(tangent-.3, 0.), (tangent+.3, 0.)]]
        case = atomic_case(parts, (0., 0., 0.), 20., pending_limit=4)
        self.assertEqual(case['preflight']['pending_cap_hits'], 1)
        self.assertGreater(case['preflight']['budget_unknown'], 0)
        self.assertEqual(case['local_gap_oracle_ink'], 6)
        self.assertTrue(case['fallback'])
        self.assertIsNone(case['commit'])
        self.assertEqual(case['displayed_ink'], 0)
        self.assertEqual(case['preflight_draw_calls'], 0)
        self.assertEqual(case['commit_candidate_visits'], 0)

    def test_streaming_preflight_matches_mask_trial(self):
        parts = [[(-2., 0.), (2., 0.)]]
        streamed = []
        mask, preflight = trial(parts, (0., 0., 0.), .1, raster=False,
                                on_segment=lambda a, b: streamed.append((a, b)))
        oracle, stats = trial(parts, (0., 0., 0.), .1)
        self.assertIsNone(mask)
        self.assertIsNone(preflight['mask_ink'])
        self.assertEqual(stats['mask_ink'], ink(oracle))
        self.assertGreater(len(streamed), 0)
        self.assertEqual({k: v for k, v in preflight.items() if k != 'mask_ink'},
                         {k: v for k, v in stats.items() if k != 'mask_ink'})

    def test_fallback_suppresses_otherwise_certified_geography(self):
        parts, site, fov, budget = fallback_cases([])['mixed-forced-budget']
        _, stats = trial(parts, site, fov, max_nodes=budget)
        self.assertEqual(stats['source_segments'], 2)
        self.assertEqual(stats['nodes'], 2)
        self.assertEqual(stats['budget_unknown'], 2)
        self.assertEqual(stats['geographic_unknown'], 2)
        self.assertEqual(stats['mask_ink'], 5)
        self.assertTrue(illustrative_fallback(stats)['arc_le_0.001_viewport_le_8'])

    def test_decoded_policy_pair_and_float_threshold_probes(self):
        parts = [[(-2., 0.), (2., 0.)]]
        paired = compare(parts, parts, (0., 0., 0.), .1)
        self.assertEqual(paired['changed_stroke_pixels'], 0)
        self.assertEqual(paired['different_fallback_examples'], [])
        self.assertEqual(paired['unquantized']['mask_ink'], 347)
        probes = synthetic_threshold_probes()
        self.assertTrue(all(p['classification']['unknown'] == 1 for p in
                            probes['front_near_tangent']))
        self.assertTrue(probes['cull_near_edge'][1]['kept'])
        self.assertFalse(probes['cull_near_edge'][2]['kept'])
        self.assertEqual(probes['viewport_near_edge'][0]['disposition'], 'unknown')
        self.assertEqual(probes['viewport_near_edge'][-1]['disposition'], 'outside')

    def test_local_gap_retains_viewport_and_budget_uncertainty(self):
        self.assertEqual(viewport_piece((1.0001, 0.), (1.0001, .1), .05)[0],
                         'unknown')
        self.assertEqual(viewport_piece((1.01, 0.), (1.01, .1), .05)[0],
                         'outside')
        self.assertEqual(viewport_piece((-2., 0.), (2., 0.), .05)[0],
                         'clipped')
        mask, stats = trial([[(-2., 0.), (2., 0.)]], (0., 0., 0.), .1)
        self.assertEqual(stats['geographic_unknown'], 0)
        self.assertEqual(stats['viewport_unknown'], 0)
        self.assertEqual(stats['mask_ink'], 347)
        self.assertEqual(ink(mask), 347)
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        _, failed = trial([[(tangent-.3, 0.), (tangent+.3, 0.)]],
                          (0., 0., 0.), 20., max_nodes=1)
        self.assertEqual(failed['budget_unknown'], 2)
        self.assertEqual(failed['mask_ink'], 0)
        self.assertTrue(all(illustrative_fallback(failed).values()))
        policy = dict(budget_unknown=0, total_unknown_span_deg=.00011,
                      viewport_unknown=0)
        self.assertTrue(illustrative_fallback(policy)['arc_le_0.0001_viewport_le_0'])
        self.assertFalse(illustrative_fallback(policy)['arc_le_0.001_viewport_le_8'])
        policy['viewport_unknown'] = 9
        self.assertTrue(illustrative_fallback(policy)['arc_le_0.001_viewport_le_8'])

    def test_work_sweep_accounts_for_each_segment_and_budget_unknown(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        parts = [[(tangent-.3, 0.), (tangent+.3, 0.)],
                 [(-2., 0.), (2., 0.)]]
        totals, unknown, budget = sweep(parts, (0., 0., 0.), 20., max_nodes=1)
        self.assertEqual(totals['source_segments'], 2)
        self.assertEqual(totals['nodes'], 2)
        self.assertEqual(totals['budget_unknown'], 2)
        self.assertEqual(totals['unknown_source_segments'], 1)
        self.assertEqual(totals['budget_source_segments'], 1)
        self.assertEqual(unknown, {(0, 0)})
        self.assertEqual(budget, unknown)
        self.assertAlmostEqual(totals['total_unknown_span_deg'], .6)
        self.assertEqual(totals['worst_segment']['part'], 0)

    def test_second_order_enclosure_reduces_graze_work_without_suppressing_unknown(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        a, b, site = (tangent, -.1), (tangent, .1), (0., 0., 0.)
        coarse = enclose(a, b, site, 20., order=1)
        refined = enclose(a, b, site, 20., order=2)
        self.assertGreater(coarse['budget_unknown'], 0)
        self.assertEqual(refined['budget_unknown'], 0)
        self.assertLess(refined['nodes'], coarse['nodes'])
        self.assertLess(refined['max_unknown_span_deg'], 1e-5)
        self.assertGreater(refined['total_unknown_span_deg'], 0)
        self.assertTrue(any(x[1] <= 0 <= y[1]
                            for x, y in refined['unknown_intervals']))
        forced = enclose(a, b, site, 20., max_nodes=1, order=2)
        self.assertEqual(forced['budget_unknown'], 2)
        with self.assertRaises(ValueError):
            enclose(a, b, site, 20., order=3)

    def test_second_derivative_and_projected_chord_bounds(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        for a, b, site in [((tangent, -.1), (tangent, .1), (0., 0., 0.)),
                           ((tangent-.3, 0.), (tangent+.3, 0.), (0., 0., 0.)),
                           ((179.6, 89.9), (-179.6, 89.9), (89.9, 179.9, 0.)),
                           ((-.1, .05), (.1, .05), (0., 0., 0.))]:
            camera = setup(site)[1]
            mid = interpolate(a, b, .5)
            center = front_value(mid, camera)
            slope = front_slope(mid, a, b, camera)
            second = front_second_bound(a, b, site[2])
            for i in range(101):
                t = i/100
                delta = t-.5
                actual = front_value(interpolate(a, b, t), camera)
                self.assertLessEqual(abs(actual-center-slope*delta),
                                     second*delta*delta/2 + 1e-7)
        a, b, site = (-.1, .05), (.1, .05), (0., 0., 0.)
        self.assertLess(chord_bound_px(.2, .1), .065)
        self.assertGreater(chord_bound_px(.2, .1), .06)
        p, q = project(a, site, .1), project(b, site, .1)
        for i in range(101):
            t = i/100
            v = project(interpolate(a, b, t), site, .1)
            for axis in (0, 1):
                self.assertLessEqual(abs(v[axis]-(p[axis]+t*(q[axis]-p[axis]))),
                                     chord_bound_px(.2, .1, 0.))
        with self.assertRaises(ValueError):
            chord_bound_px(-1, .1)

    def test_visibility_enclosure_crossing_graze_and_budget_fail_closed(self):
        from coastline_preflight import A
        tangent = math.degrees(math.acos(A/(A+HEIGHT)))
        crossing = enclose((tangent-.3, 0.), (tangent+.3, 0.), (0., 0., 0.), 20.)
        self.assertGreater(crossing['front'], 0)
        self.assertGreater(crossing['back'], 0)
        self.assertTrue(any(a[0] <= tangent <= b[0]
                            for a, b in crossing['unknown_intervals']))
        self.assertEqual(crossing['budget_unknown'], 0)
        graze = enclose((tangent, -.1), (tangent, .1), (0., 0., 0.), 20.)
        self.assertGreater(graze['budget_unknown'], 0)
        self.assertGreater(graze['total_unknown_span_deg'], 0)
        self.assertTrue(any(a[1] <= 0 <= b[1]
                            for a, b in graze['unknown_intervals']))
        exhausted = enclose((-2., 0.), (2., 0.), (0., 0., 0.), .1, max_nodes=1)
        self.assertEqual(exhausted['nodes'], 1)
        self.assertEqual(exhausted['budget_unknown'], 0)  # certified front
        exhausted = enclose((tangent-.3, 0.), (tangent+.3, 0.),
                            (0., 0., 0.), 20., max_nodes=1)
        self.assertEqual(exhausted['budget_unknown'], 2)
        self.assertAlmostEqual(exhausted['total_unknown_span_deg'], .6)

    def test_visibility_bound_encloses_sampled_front_values(self):
        for a, b, site in [((80., 0.), (90., 0.), (0., 0., 0.)),
                           ((179.6, 89.9), (-179.6, 89.9), (89.9, 179.9, 0.)),
                           ((-89., 0.), (89., 0.), (0., 0., 0.))]:
            camera = setup(site)[1]
            mid = interpolate(a, b, .5)
            radius = front_lipschitz(a, b, site[2]) / 2
            center = front_value(mid, camera)
            for i in range(101):
                self.assertLessEqual(abs(front_value(interpolate(a, b, i/100), camera)-center),
                                     radius + 1e-7)

    def test_quantization_rejects_shortest_longitude_branch_change(self):
        check_shortest_longitude_branches([[(0, 0), (179.99999, 0)]],
                                           [[(0, 0), (180., 0)]])
        with self.assertRaises(ValueError):
            check_shortest_longitude_branches([[(0, 0), (180.00001, 0)]],
                                               [[(0, 0), (180., 0)]])

    def test_trial_coordinate_bound_and_coverage_threshold(self):
        bound = vertex_screen_bound(.00005)
        self.assertLess(bound['onscreen_axis_displacement_px'], .02)
        self.assertGreater(bound['onscreen_axis_displacement_px'], .01)
        for site in (SITES['San Francisco'], SITES['north polar'],
                     SITES['equator/dateline']):
            observer, _, east, north, up = setup(site)
            span = math.tan(math.radians(.1/2))
            point = (site[1], site[0])
            perturbed = (point[0] + .00005, point[1] + .00005)
            def screen(p):
                xyz, _ = surface(*p)
                delta = tuple(xyz[i]-observer[i] for i in range(3))
                depth = HEIGHT - dot(delta, up)
                return (176+117/span*dot(delta, east)/depth,
                        120-117/span*dot(delta, north)/depth)
            a, b = screen(point), screen(perturbed)
            self.assertLessEqual(max(abs(x-y) for x, y in zip(a, b)),
                                 bound['onscreen_axis_displacement_px'])
        below, _ = stroke([((50., 120.49), (60., 120.49))])
        above, _ = stroke([((50., 120.51), (60., 120.51))])
        self.assertGreater(difference(below, above), 0)

    def test_continuous_stroke_is_split_invariant_and_clipped(self):
        whole, _ = stroke([((2., 120.), (350., 120.))])
        split, _ = stroke([((2., 120.), (120.375, 120.)),
                           ((120.375, 120.), (350., 120.))])
        self.assertEqual(difference(whole, split), 0)
        for y in range(240):
            for x in range(400):
                if whole.getpixel((x, y)) == 0:
                    self.assertTrue(BOUNDS[0] <= x <= BOUNDS[1]
                                    and BOUNDS[2] <= y <= BOUNDS[3])
        self.assertEqual(whole.getpixel((176, 120)), 0)
        self.assertEqual(whole.getpixel((176, 121)), 255)
        self.assertEqual(difference(stroke([((0., 0.), (2., 2.))])[0],
                                    stroke([])[0]), 0)

    def test_both_hidden_endpoints_can_enclose_visible_run(self):
        segments = []
        _, stats = draw([[(-89., 0.), (89., 0.)]], (0., 0., 0.), 5.,
                        sample_step=180., segments_out=segments)
        self.assertEqual(stats['interior_front'], 1)
        self.assertEqual(stats['tangents'], 2)
        self.assertGreater(sum(p == 0 for p in stroke(segments)[0].get_flattened_data()), 0)

    def test_stroke_keeps_visible_middle_and_edge_crossing(self):
        for part, site, fov in [([[(-2., 0.), (2., 0.)]], (0., 0., 0.), .1),
                                ([[(-89., 0.), (89., 0.)]], (0., 0., 0.), 5.)]:
            masks, _, stats = scene(part, site, fov)
            self.assertGreater(stats['steps']['0.2']['ink'], 0)
            self.assertEqual(difference(masks[.2], masks[.01]), 0)

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
