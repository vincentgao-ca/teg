import React, { useMemo } from 'react'
import { Formik, Field, Form } from 'formik'
import { TextField } from 'formik-material-ui'
import { animated, useSpring } from 'react-spring'
import Measure from 'react-measure'
import { Query, Mutation } from 'react-apollo'
import classNames from 'classnames'
import gql from 'graphql-tag'

import {
  Typography,
} from '@material-ui/core'

import Typeahead from '../../common/Typeahead'
import Loading from '../../common/Loading'

import ButtonsFooter from '../ButtonsFooter'

import FormikSchemaForm from '../../pages/connected/config/components/FormikSchemaForm/index'
import transformComponentSchema from '../../pages/connected/config/printerComponents/transformComponentSchema'

import useSchemaValidation from '../../pages/connected/config/components/FormikSchemaForm/useSchemaValidation'

const CREATE_MACHINE = gql`
  mutation($input: CreateMachineInput!) {
    createMachine(input: $input) {
      errors { message }
    }
  }
`

const schemaWithoutDef = ({ schema }) => {
  const {
    machineDefinitionURL,
    ...properties
  } = schema.properties

  const required = schema.required.filter(fieldName => (
    fieldName === 'machineDefinitionURL'
  ))

  return {
    properties,
    required,
  }
}

const Step3SetupForm = ({
  classes,
  machineDefinitionURL,
  setMachineDefinitionURL,
  suggestions,
  devices,
  className,
  loadingMachineSettings,
  machineSettingsError,
  schemaForm,
  history,
  location,
}) => {
  const machineIsSet = machineDefinitionURL != null

  const configSpring = useSpring({ x: machineIsSet ? 1 : 0 })

  const machineDefName = useMemo(() => {
    if (!machineIsSet) return null

    const suggestion = suggestions.find((suggestion) => (
      suggestion.value === machineDefinitionURL
    ))
    return suggestion.label
  }, [machineDefinitionURL, suggestions])

  const { schema = {}, form } = useMemo(() => {
    if (schemaForm == null) {
      return {}
    }

    return {
      schema:  transformComponentSchema({
        schema: schemaWithoutDef(schemaForm),
        materials: [],
        devices,
      }),
      form: schemaForm.form,
    }
  }, [schemaForm])

  const validate = useSchemaValidation(schemaForm)

  return (
    <Mutation
      mutation={CREATE_MACHINE}
      onCompleted={() => {
        history.push(`/get-started/4${location.search}`)
      }}
    >
      {(addPrinter, { loading, data }) => (
        <Formik
          enableReinitialize
          initialValues={{
            machineDefinitionURL,
            ...Object.keys(schema.properties || {}).reduce((acc, k) => {
              acc[k] = null
              return acc
            }, {}),
          }}
          validate={validate}
          onSubmit={async (values, { setSubmitting }) => {
            const input = {
              model: values,
            }
            try {
              await addPrinter({ variables: { input } })
            } catch (e) {
              setSubmitting(false)
              alert(e)
            }
          }}
        >
          {({ values, isSubmitting }) => (
            <Form className={classes.form}>
              <div
                className={classNames([
                  className,
                  classes.stretchedContent,
                ])}
              >
                <div className={classes.root}>
                  <div className={classes.part1}>
                    <Measure bounds>
                      {({ measureRef, contentRect: { bounds } }) => (
                        <animated.div
                          style={{
                            height: configSpring.x
                              .interpolate({
                                range: [0, 0.5, 1],
                                output: [bounds.height, bounds.height, 0],
                              })
                              .interpolate(x => `${x}px`),
                            opacity: configSpring.x.interpolate({
                              range: [0, 0.5],
                              output: [1, 0],
                            }),
                          }}
                        >
                          <div className={classes.introText} ref={measureRef}>
                            <Typography variant="h6" paragraph>
                              Great! we've connected to your Raspberry Pi!
                            </Typography>
                          </div>
                        </animated.div>
                      )}
                    </Measure>
                    <Typography variant="body1" paragraph>
                      Now, what kind of 3D Printer do you have?
                    </Typography>
                    <Typeahead
                      suggestions={suggestions}
                      name="machineDefinitionURL"
                      label="Search printer make and models"
                      onChange={setMachineDefinitionURL}
                    />
                  </div>
                  <animated.div
                    className={classes.config}
                    style={{
                      flex: configSpring.x.interpolate({
                        range: [0, 0.5],
                        output: [0, 1],
                      }),
                      opacity: configSpring.x.interpolate({
                        range: [0, 0.5, 1],
                        output: [0, 0, 1],
                      }),
                    }}
                  >
                    { machineIsSet && (() => {
                      if (loadingMachineSettings) {
                        return (
                          <Loading className={classes.loadingPart2}>
                            Loading Printer Settings...
                          </Loading>
                        )
                      }
                      if (machineSettingsError != null) {
                        return (
                          <div>
                            <h1>Error</h1>
                            {JSON.stringify(machineSettingsError)}
                          </div>
                        )
                      }

                      return (
                        <React.Fragment>
                          <Typography variant="body1" paragraph>
                            { 'aeiouAEIOU'.includes(machineDefName[0]) ? 'An' : 'A'}
                            {' '}
                            <b>
                              {machineDefName}
                            </b>
                            ?
                            {' '}
                            Great. We just need a bit of information to get it set up.
                          </Typography>
                          <FormikSchemaForm
                            schema={schema}
                            form={form}
                            path=""
                            className={classes.configForm}
                          />
                        </React.Fragment>
                      )
                    })()}
                  </animated.div>
                </div>
              </div>
              <ButtonsFooter
                step={3}
                disable={machineIsSet === false || isSubmitting}
                type="submit"
                component="button"
                history={history}
              />
            </Form>
          )}
        </Formik>
      )}
    </Mutation>
  )
}

export default Step3SetupForm
