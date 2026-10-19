# Terrarium

> terrarium /tə-râr′ē-əm/

> noun:
>   A small enclosure or closed container in which selected living
>   plants and sometimes small land animals, such as turtles and lizards,
>   are kept and observed.

## Overview:
The plan is to create a self-sustaining ecosystem of _'creatures'_ 
using RPG-like stats as _"genes"_ and observe the consequences
in the form of strategies used for surviving, behavioural tendencies,
predator-prey relationships, etc.

## Goal:
To study speciation, and specialisation of certain traits, possible
emergent strategies for survival, natural tendencies of creatures,
with a target of surviving and reproducing; similar to how it is in real life.

## Brief:

Each creature has a set of crystallised _"genes"_[^1]:

- size
- strength : carrying capacity
- max speed
- diet : how efficiently plants v/s carcasses can be consumed to get energy
- vision
- maturity : minimum age and amount of energy to reproduce
- intelligence : quality of decision making skills:
    - ability to makeout fresh food from stale.
    - fight or flight
    - memory capacity, object permanence
    - ???

There are also _"live stats"_:

- energy
    spent for movement, thinking, and a decaying _"hunger"_,
    creature dies when energy hits zero
- gestation
    concieving a child incurs an energy cost, a
    _"pregnant creature"_[^2] spends extra energy
- age
    with increasing age strength, speed, vision and ability to
    maturity could change.

Apart from the living creatures[^3] there is also a concept of _"regions"_
that could be introduced: in cold regions, carcasses and plants decay at
a slower rate, energy is consumed faster.  The energy cost for conception
is also dependent on region. In the future if creatures can pick-up and
drop items/plants/carcasses maybe strategies of preserving dead bodies
could develop.[^4]

To make it possible for group/team strategies to emerge, species need
to be able to identify their own _"(kin recognision)"_, this is also
required for mating.  Also to make it easier to visualize for ourselves,
procedurally generated sprites can be used. Maybe vision and intelligence
can be used in another model for species to recognise each other, and
possible threats from apex predators.

[^1]: A list of genes and live stats is to be finalized with further discussion. An attempt is made
      to find the least implementation difficulty for maximum (interesting) emergent
      behaviour.
      Also as it maybe obvious; the genes aren't truly _"crystallized"_, there is a particular
      _"fluid"_ component as well, affected by age and other live stats.

[^2]: No concept of sex, make everyone hermaphroditic.

[^3]: Simulating plants is deemed to be out of scope here, as of now plants will
      drop randomly based on a regions fertility. Which could increase if more
      decays occur in said region and its other stats.

[^4]: Few more genes could be introduced to get further specialisation of
      species in said regions, or existing live stats like rate of increase
      of age could be modified as a hack.
