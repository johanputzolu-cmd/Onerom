## picotool Specification Deficiencies

- picotool requests a dTransferLength of 256 bytes on GET_INFO command.  The spec states:

  "dTransferLength the size of data to be received. Note this must be a multiple of 4, and less than 256"

  This is a clear bug, either in picotool or the specification as 256 is not less than 256.

- picotool expects the picoboot vendor interface to be interface 0 (if the descriptor contains a single interface) or 1 (if the descriptor contains two interfaces).  The spec states:

  "Don’t rely on the interface number, because that is dependent on whether the device is currently exposing the Mass Storage Interface."

  It seems clear that picotool has assumed there will only be two interfaces, and it could be argued that picotool is strictly followed the spec, albeit unhelpfully.  However, it is hard to correlate what the spec says ("don't rely on the interface number") with its behaviour (relying on the interface number being 0 or 1).  If the interface will always be 0 or 1, the spec should say so, so other tools and implementations can rely on the same assumption.