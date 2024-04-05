# Jank

I'm using a modified (mostly) ansi-compatible swedish dvorak layout, special sign like `;` and `:` should 
be on the same key with the difference being shifting.

This requires some dirty hacks where shift is artificially inserted and removed when these are pressed.

However, this produces jank if another key is pressed before the jank-key is released.

This isn't unique to this project, as the same thing would happen with any custom layering in `qmk` in the case 
that the permanent layer is changed while a key activating a temp-layer is pressed, if that key activating 
the temp-layer is different, the key-up will produce jank.  

A list of the danger cases follows:

1. Any modifier (e.g. shift) is temporarily inserted to produce a symbol out, another key is pressed before that 
temp shift can be cleared by the key-up. (can't be cleared immediately because then holding a key wouldn't produce
repeated keypresses).
2. Any modifier (e.g. shift) is temporarily removed to produce a symbol out, same as above but reversed.
3. Any layer is pressed, when the button activating that is released, the meaning of that button has changed 
to something different resulting in an unexpected key-up.
4. Any button is pressed, then the layer is changed and when that button goes key-up the meaning of that button is changed.

Examples of jank produced by these cases are:

1. Shift suddenly stops being pressed.
2. Shift suddenly starts being pressed.
3. A random key MAY be popped depending on circumstance.
4. A random key MAY be popped depending on circumstance.

## Solutions

Robust solutions are a bit hackish, the best one may be sequence-counting and temp-modifiers as first class.

## Solution for 1 and 2 

When a temporary modifier is added, save the unmodified state. If another key is pressed 
(pressed only, don't care about releases), that key will send a report using the previous mods.

All presses increments the seq-count on key-press. 

Keys with temp-mods save their sequences, only resets them if they are the next exact sequence.

### Caveat

No need to care about key-releases since the last key pressed is still repeated if an old key is released.

This doesn't quite hold true for modifiers, but it can be special-cased if necessary

## Solutions for 3 and 4

Keyboard pop-layer behaviour can be corrected if the last used layer is stored with the key, if the current layer 
doesn't match make that key-up a no-op, alternatively, make it a pop-key for the key it would have been. 
Second might even be cleaner code-wise, and a bit better.