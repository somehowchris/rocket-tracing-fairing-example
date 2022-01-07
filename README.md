# Rocket Tracing Fairing Example
[![Open in Gitpod](https://gitpod.io/button/open-in-gitpod.svg)](https://gitpod.io/#https://github.com/somehowchris/rocket-tracing-fairing-example)

This repository aims to give a short example of how you can add a Fairing to your Rocket for tracing and how to use it in requests.
> As Rocket currently doesn't implement this by default many have asked about implementing this. The actix-web tracing crate has been taken as a reference for the info span data


### Why isn't this a crate?

As of right now I have has 4 people asking me the same question about tracing and rocket from difference perspektives within 4 days. If more people point out their desire to have a crate for this please open an issue and let the people vote