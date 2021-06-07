#pragma once

#include <memory>
#include <bitcoin/system.hpp>

namespace darkfi {

std::unique_ptr<libbitcoin::system::wallet::hd_private> new_private_key() {
    return std::make_unique<libbitcoin::system::wallet::hd_private>();
}

} // namespace wallet
