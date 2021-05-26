// This script reads a set of Starport authoritites

import Starport from 0xf8d6e0586b0a20c7

pub fun main(): [String] {

    let authorities = Starport.getAuthorities()

    return authorities
}