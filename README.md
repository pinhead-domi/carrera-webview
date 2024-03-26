# Carrera Webview
## Features
This project aims to bring information the carrera ditial 1:32 slotcat track to your browser, live! In order to do so, this rust webserver, utilising the axum framework, communicates with an arudino via a serial port. The messages recieved from the arduino will then be mapped to a stream and forwarded to each connected client via the SSE protocol, enabling real-time updates without the need for polling.

Right now the server offers:
 - A rudimentary user interface written in HTML5, fetching data via SSE and displaying them using JQuery
 - Start-Light events
 - New-Lap events
 - Fuel updates
 - Controller-Speed updates
 - Reset events

with the following messages planned:
 - Pitlane-Status updates
 - Safety/PaceCar-Status updates
 - Additional Information from accessories like the sector-timme-sensors need changes from the arduino side of the project

## Roadmap
The most important part for me was to get SSE in combination with serial communication to work, which gives a solid foundation to build uppon. From here there are a few priorities that I think need to be adressed in a similar order as follows:
 1. Refactor the codebase [seperate files, variable names, error handling/removing unwraps]
 2. Iron out bugs/oversights
  - First lap always has a time of 0s
  - Car states are not implemented as an axum state
  - Handle reset events from the CU better
 3. Add api endpoints without SSE [requires axum state from previous points] to keep [new] clients in sync
 4. Major UI 'improvements': The current UI is just a POC, the idea is to implement the following:
  - Starting lights as a popup/overlay
  - Track-Status indicator [Green flag/Safety car]
  - Driver position tower
  - Car info cards containing completed laps, personal best laptime, last laptime, fuel level, pit status, driver name, controller-speed indicator
  - Fastest lap popup
  - Optimized mobile view [select one driver]

## Ideas for the future
These ideas are just fun things that I think could be done without too much difficulty, but have no real priority or basis in any way:
 - Lap-telemetry: The server keeps track off all controller-speed changes with timestamps for the last/best lap that a controller has driven. This would enable a telemetry graph that the frontend can display using something like Chart.js
