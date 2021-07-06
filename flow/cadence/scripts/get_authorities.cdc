// This script reads a set of Starport authoritites

import Starport from 0xc8873a26b148ed14

pub fun main(): [String] {

    let authorities = Starport.getAuthorities()

    return authorities
}