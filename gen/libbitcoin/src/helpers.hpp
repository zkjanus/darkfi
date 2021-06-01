#pragma once

#include <memory>
#include <bitcoin/system.hpp>
#include "rust/cxx.h"


namespace libbitcoin {

namespace system {

std::unique_ptr<hd_private> new_private_key(seed) {
  return std::make_unique<hd_private>(seed);
}

}

}
