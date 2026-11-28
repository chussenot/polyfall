# A pause in the middle of the biggest sound: everything stops but the pause cue; then resume.
# seconds: 8
# needs: stage
# expect: lock+hard_drop+clear_4+level_up | pause | resume
#
# In the timeline, look at the theme's volume: lowered under the clear, faded out by the pause,
# faded back in by the resume, and not lowered any more by a clear that was cut short.

rest 1.2
press F6 F4;  rest 0.15;  press space;  rest 0.35
press p;  rest 1.5
press p;  rest 2.5
