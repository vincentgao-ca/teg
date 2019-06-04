import React, { useState } from 'react'
import { compose } from 'recompose'
import {
  Card,
  CardContent,
  Grid,
} from '@material-ui/core'
import {
  ArrowUpward,
  ArrowDownward,
} from '@material-ui/icons'

import useJog from '../../../../../common/useJog'

import JogButton from './JogButton'
import JogDistanceButtons from './JogDistanceButtons'

const ZJogButtons = ({ printer }) => {
  const distanceOptions = [0.1, 1, 10]
  const [distance, onChange] = useState(distanceOptions[0])

  const jog = useJog({ printer, distance })

  return (
    <Card>
      <CardContent>
        <Grid
          container
          spacing={24}
        >
          <JogButton xs={12} onClick={jog('z', 1)}>
            <ArrowUpward />
          </JogButton>
          <JogButton xs={12} disabled>
            Z
          </JogButton>
          <JogButton xs={12} onClick={jog('z', -1)}>
            <ArrowDownward />
          </JogButton>
          <JogDistanceButtons
            distanceOptions={distanceOptions}
            input={{
              value: distance,
              onChange,
            }}
          />
        </Grid>
      </CardContent>
    </Card>
  )
}

export default enhance(ZJogButtons)
