# FDE CLI script for the led_all_on recipe.
# Run from CLI with: source led_all_on.tcl

scan_recipes
discover
mount 0
load_recipe led_all_on
program 0
