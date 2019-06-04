import React, { useState } from 'react'
import { compose } from 'recompose'
import {
  Grid,
  Button,
} from '@material-ui/core'

import useJog from '../../../../../common/useJog'

import JogDistanceButtons from '../jog/JogDistanceButtons'

const ExtruderButtons = ({
  printer,
  address,
  disabled,
  extrudeColor = 'primary',
  customButton,
  showRetract = true,
  showExtrude = true,
}) => {
  const distanceOptions = [0.1, 1, 10, 50, 100]
  const [distance, onChange] = useState(distanceOptions[2])

  const jog = useJog({ printer, distance })

  return (
    <Grid
      container
      spacing={8}
    >
      <Grid item sm={6}>
        <JogDistanceButtons
          distanceOptions={distanceOptions}
          input={{
            value: distance,
            onChange,
          }}
        />
      </Grid>
      <Grid item sm={6}>
        <div style={{ textAlign: 'right' }}>
          { customButton }
          { customButton && (showExtrude || showRetract) && (
            <div style={{ display: 'inline-block', width: '16px' }} />
          )}
          { showRetract && (
            <Button
              variant="contained"
              disabled={disabled}
              onClick={jog(address, -1)}
            >
              Retract
            </Button>
          )}
          { showExtrude && showRetract && (
            <div style={{ display: 'inline-block', width: '16px' }} />
          )}
          { showExtrude && (
            <Button
              variant="contained"
              color={extrudeColor}
              disabled={disabled}
              onClick={jog(address, 1)}
            >
              Extrude
            </Button>
          )}
        </div>
      </Grid>
    </Grid>
  )
}

export default enhance(ExtruderButtons)
