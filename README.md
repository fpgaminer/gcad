# GCad

**WARNING: WIP**

A scripting language inspired by OpenSCAD for making quick and dirty CNC programs.  Want to make a quick set of counterbored holes?  Here's an example:

```
board_thickness = 12.2mm;
board_height = 812.5mm - 6.35mm/2;
board_width = 5in;

cutter_diameter(6.35mm);
material('BALTIC_BIRCH_PLYWOOD');

comment('Holes for threaded inserts for ceiling brackets');
for y in linspace(1.5in, board_height - 1.5in, 2) {
	for x in linspace(3/4in, 3.25in, 2) {
		comment('Counterbore');
		circle_pocket(x, y, radius=6.35mm, depth=3mm);
		comment('Threaded insert hole');
		circle_pocket(x, y, radius=4.75mm, depth=board_thickness);
	}
}


// Hole which the LED strip wires pass through
material('ALUMINUM');
comment('LED strip wire hole');
drill(groove_x + groove_width / 2, board_height - 1mm, board_thickness);
```

Then: `gcad -o output.nc input.gcad`

Done!