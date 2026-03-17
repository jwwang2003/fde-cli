# FDE CLI script for the name_display recipe.
# Run from CLI with: source name_display.tcl

scan_recipes
discover
mount 0
load_recipe name_display
program 0
attach 0

# Drive a few idle ticks so outputs can stabilize.
tick 10 0x0400

# Keep session attached for manual interaction.
# Use `detach` manually when done.
