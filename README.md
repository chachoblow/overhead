# Overhead radar lab

A design sandbox for the radar view, kept apart from the production code: this
is an orphan branch (`radar-lab`) with no history shared with `main`. Nothing
here is built, tested or shipped. It exists to try radar-view ideas quickly,
pick one, and only then implement it in the workspace crates.

- `radar-lab.html`: the whole lab as one self-contained page. It is published
  privately as a claude.ai artifact:
  <https://claude.ai/artifact/CA3Yhyz2qSfoDRpsG8XsQA>. Open the file directly
  in a browser to use it offline.
- `render.js`: renders any configuration headlessly to a PNG with Node (no
  dependencies), using the page's own drawing code.

## What the lab is

A 400×240 1-bit canvas drawn pixel for pixel, with the real 6×10 font
(embedded-graphics), the real coastline asset
(`render/assets/coastline-110m-40km.bin`) and the real `visual` group fixture
(`firmware/fixtures/celestrak-visual-20261103.json`, 156 objects). Observer
39.007, -104.883, 2.187 km; starts 2026-11-03T02:00Z. The orbital view puts
the camera 75,000 km above the observer, north up.

Differences from the device, so screenshots aren't mistaken for it:

- Orbits use two-body propagation from the OMM elements, not SGP4: close, not
  exact.
- Every object is recomputed every frame. The device recomputes the selected
  object every frame and the rest within 1 s (decision 0077), so unselected
  markers and their trails step about once a second there.
- The page shows the panel at 2× in a browser; legibility at the real
  0.15 mm pixel pitch is only judged on the board.

## Rendering headlessly

```sh
node render.js "Favourite C" --out fav-c.png                 # a preset by name
node render.js "marker=diamondfill7r below=pluscornershalo" --fov 20 --minutes 0
```

`--fov` is in degrees (smaller is zoomed in; default 8), `--minutes` offsets
the start time (default 4), `--sel` picks a NORAD ID, `--scale` sets the PNG
pixel scale (default 2). A config line sets only the keys it names; the rest
come from the lab's defaults (`CURRENT`), which model the device as it was.

## Editing the lab

Edit `radar-lab.html` here and republish it to the same artifact URL (in a
Claude session: Artifact publish with this file path and that `url`). The
page's option groups live in `GROUPS`, defaults in `CURRENT`, presets in
`PRESETS`. The first `<script>` holds the embedded data (`SATS`, `COAST`,
`FONT6X10`); the second holds the code. Everything after
`// ---------- UI ----------` is browser-only; `render.js` evaluates the code
before it.

## Favourites

Each is a preset in the page. **C is the latest** and the candidate to take
to the board next.

**Favourite C** (2026-11-22)

```
proj=orbital strip=pills inset=off eltrend=none pass=none pillpos=left marker=diamondfill7r below=pluscornershalo earthout=sphere1000deepoutlinefade halo=on trail=line sel=crosspulse sellabel=hide path=orbitsolid lstyle=box labels=selected earth=coast night=none npat=sparse space=none extras=none cue=scale arrows=halo
```

- Satellites above the horizon: 7 px octagon (rounded filled diamond) with a
  1 px cleared halo, plus a short past-line motion cue.
- Satellites below the horizon (the rest of the group): four pixels in a plus
  layout with no centre, cleared halo. Never selectable, no off-screen arrows,
  hidden when behind the Earth.
- Earth: full tone inside the ring where 1,000 km satellites are above the
  horizon; just past it a quick drop to 70%, then sphere shading down to 12%
  at the limb. The outline stays solid. Coastline, no night dither.
- Selected satellite: crosshair through it (its own marker in the centre) plus
  a pulsing ring, no ID label, full orbit as a solid line, no motion cue (the
  orbit shows its motion).
- Split light pills on the left, sized to content; simple scale ruler on a
  pill-shaped light backing; off-screen arrows with a cleared halo. No centre
  cross, no "N".

**Favourite A / B** (2026-11-20): left pills, 11 px panel-satellite sprite
turning with heading (8 ways) with a cleared halo, crosshair + pulsing ring,
dotted orbit, sparse night dither, scale ruler, arrows. A: no motion cue, ID
label shown. B: past-line motion cue, ID label hidden.

## Device notes for Favourite C

- **Below-horizon markers cost no propagation.** `RadarApp` already samples
  every object; `Sample::above_horizon` only filters drawing
  (`app/src/lib.rs`). They need a behind-the-Earth test (a ray–sphere check per
  object) that above-horizon markers never needed. They do change the view's
  meaning from "only what is above you" to "the whole group, with what is
  above you stressed", so this needs a decision record.
- **The octagon has no heading**, so no direction estimate and no hysteresis
  (both were needed for the turning panel sprite).
- **Sphere shading** is per pixel. With the camera straight above the
  observer the sky ring is an exact screen circle, so the tone is a function
  of screen distance from the centre plus an 8×8 ordered-dither lookup; the
  lab's per-pixel ray cast is not needed on the device.
- **The selected orbit** is about 160 propagations (~350 ms on the S3 at
  ~2.2 ms each); compute it on selection and refresh every few seconds, not
  per frame.
- **Past-line trails** need a few stored past positions per object; with the
  1 s stagger they grow in visible steps.
- **Check on the panel:** the 4-pixel below-horizon plus, the solid orbit
  line, the 12% tone at the limb, and the 7 px octagon at 0.15 mm pitch.

## Ideas tried and set aside

Useful so they aren't re-proposed: bordered right strip; alternating
black/white scale-bar segments; a ruler outline; the centre cross; the "N"
label; patterned space backgrounds (stars, glow and dark space were added
later as options, none chosen); dots and chevrons as main markers (lost in
dither and coastline); thin heading or tail lines; the detailed satellite
sprite; a sky window fading the map outside the horizon area (the camera is
straight above, so it is only a centred disc); rising/setting cues; a pass
progress bar (pass prediction is deferred by decision 0077; "sets in" and an
elevation-trend mark remain options); a sky dome restyled with the map;
switching to the dome by zoom; solid or ring-shaped below-horizon markers
(too close to the above-horizon ones; light open point-sets work).

## Next step

Throwaway board test: implement Favourite C on a branch off `main` (not this
one) in `app`/`render`, take simulator snapshots, flash it through
`firmware/mac.sh`, and judge it on the Sharp panel. If it holds up, write the
decision (narrowing what the orbital view shows) and a work item, per
`AGENTS.md`.
