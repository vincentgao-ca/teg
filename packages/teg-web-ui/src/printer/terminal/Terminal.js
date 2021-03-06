import React from 'react'
import { withFormik, Form, Field } from 'formik'
import { TextField } from 'formik-material-ui'

import Button from '@material-ui/core/Button'
import Typography from '@material-ui/core/Typography'
import Link from '@material-ui/core/Link'

import gql from 'graphql-tag'

import useExecGCodes from '../_hooks/useExecGCodes'
import useLiveSubscription from '../_hooks/useLiveSubscription'

import TerminalStyles from './TerminalStyles'

const GCODE_HISTORY_SUBSCRIPTION = gql`
  subscription DevicesSubscription($machineID: ID!) {
    live {
      patch { op, path, from, value }
      query {
        machines(machineID: $machineID) {
          id
          status
          gcodeHistory(limit: 200) {
            id
            direction
            createdAt
            content
          }
        }
      }
    }
  }
`

const enhance = withFormik({
  mapPropsToValues: () => ({
    gcode: '',
  }),
})

const Terminal = ({
  match,
  values,
  resetForm,
}) => {
  const classes = TerminalStyles()
  const { machineID } = match.params

  const onSubmit = useExecGCodes((e) => {
    e.preventDefault()
    resetForm()

    return {
      machineID,
      gcodes: [values.gcode],
    }
  })

  const {
    data,
    loading,
    error,
  } = useLiveSubscription(GCODE_HISTORY_SUBSCRIPTION, {
    variables: {
      machineID,
    },
  })

  if (loading) {
    return <div />
  }

  if (error) {
    throw error
  }

  const { status, gcodeHistory } = data.machines[0]
  const isReady = ['READY'].includes(status)
  // const isReady = ['READY', 'PRINTING'].includes(status)

  return (
    <div className={classes.root}>
      <Form className={classes.inputRow} onSubmit={onSubmit}>
        <Field
          className={classes.input}
          label="GCode"
          name="gcode"
          component={TextField}
          disabled={!isReady}
        />
        <Button
          variant="contained"
          type="submit"
          disabled={!isReady}
        >
          Send
        </Button>
      </Form>
      <Typography
        variant="body2"
        className={classes.reference}
        component="div"
      >
        Need a GCode reference? Try the
        {' '}
        <Link
          href="https://marlinfw.org/meta/gcode/"
          underline="always"
          target="_blank"
          rel="noopener noreferrer"
        >
          Marlin GCode Index.
        </Link>
      </Typography>
      <Typography
        variant="body2"
        className={classes.terminalHistory}
        component="div"
      >
        {
          [...gcodeHistory].reverse().map(entry => (
            // eslint-disable-next-line react/no-array-index-key
            <div
              key={entry.id}
              className={[
                classes.terminalEntry,
                classes[entry.direction === 'TX' ? 'tx' : 'rx'],
              ].join(' ')}
            >
              {
                /*
                <span className={classes.createdAt}>
                  {entry.createdAt}
                </span>
                */
              }
              <span className={classes.direction}>
                {` ${entry.direction} `}
              </span>
              <span className={classes.content}>
                {entry.content}
              </span>
            </div>
          ))
        }
      </Typography>
    </div>
  )
}

export default enhance(Terminal)
