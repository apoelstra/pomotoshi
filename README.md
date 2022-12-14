# Pomotoshi

This project is a [Pomodoro timer](https://www.raptitude.com/2021/11/how-to-do-things/)
for xmobar strongly inspired by [pomobar](https://github.com/rlcintra/pomobar). It
works in essentially the same way (you hook it into xmobar and then use dbus-send
commands to talk to it) but has a slightly different feature set:

* It supports only one block length rather than an array of them; Win+1 starts the timer
* Trying to restart the timer will flash an error rather than restarting/cycling through times.
  You need to send a "cancel" command before restarting
* After every block there is a 5 minute (hardcoded, TODO make it configurable) "cooldown"
  period during which time you cannot start a new block.
* Using `xdotool getwindowfocus getwindowname`, the tool records statistics on what you
  are focused on, during blocks. To get statistics you can call dbus-send with the
  `dumpStats` or `dumpLongStats` commands. These take a boolean argument saying whether
  or not to reset the counters. FIXME you should be able to disable this. It also uses
  a bunch of heuristics to organize activities, which I don't have any real intention
  of making more general.
* Right now we hardcode "keyboard.sh" rather than having a `--terminatedShellCmd` option.
  This makes the tool basically unusable except for me, it's a TODO to fix this.
* The colors fade (also hardcoded, FIXME should be configurable)

# Setting up

Just like with pomodoro, to install it, add a line to your xmobarrc like

    Run CommandReader "~/bin/pomotoshi" "pomobar"

and then to use it, edit your keylist in your xmonad.hs like

     , ((XMonad.modMask conf, xK_1), spawn "dbus-send --print-reply --dest=org.Pomotoshi /org/pomotoshi org.Pomotoshi.startBlock uint64:1500")
     , ((XMonad.modMask conf, xK_2), spawn "dbus-send --print-reply --dest=org.Pomotoshi /org/pomotoshi org.Pomotoshi.pauseBlock")
     , ((XMonad.modMask conf, xK_3), spawn "dbus-send --print-reply --dest=org.Pomotoshi /org/pomotoshi org.Pomotoshi.cancelBlock")


