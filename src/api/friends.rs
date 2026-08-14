use super::localplayer::PlayerSteamId;
use napi::bindgen_prelude::BigInt;
use napi_derive::napi;

/// What a friend is playing, as far as Steam will say.
///
/// `gameId` is the raw 64 bit game id rather than an app id: the app id is the
/// low 32 bits of it, and handing the raw value over loses nothing while
/// leaving the caller free to be correct about which bits those are.
///
/// `lobby` is the lobby that friend is in, and it is absent rather than zero
/// when there is none. It is the reason this exists — a lobby id is what you
/// join a friend through, and without it there is no way to find one that does
/// not start with them inviting you.
#[derive(Debug)]
#[napi(object)]
pub struct FriendGamePlayed {
    pub game_id: BigInt,
    pub lobby: Option<BigInt>,
}

#[derive(Debug)]
#[napi(object)]
pub struct SteamFriend {
    pub steam_id: PlayerSteamId,
    pub name: String,
    /// One of `offline`, `online`, `busy`, `away`, `snooze`, `lookingToTrade`
    /// or `lookingToPlay`.
    pub state: String,
    /// Absent when they are not in a game.
    pub game_played: Option<FriendGamePlayed>,
}

#[napi]
pub mod friends {
    use super::{FriendGamePlayed, PlayerSteamId, SteamFriend};
    use napi::bindgen_prelude::{BigInt, Buffer};
    use steamworks::{FriendFlags, FriendState, SteamId};

    fn state_name(state: FriendState) -> String {
        let name = match state {
            FriendState::Offline => "offline",
            FriendState::Online => "online",
            FriendState::Busy => "busy",
            FriendState::Away => "away",
            FriendState::Snooze => "snooze",
            FriendState::LookingToTrade => "lookingToTrade",
            FriendState::LookingToPlay => "lookingToPlay",
        };
        name.to_string()
    }

    /// The people on this player's friends list.
    ///
    /// `flags` is a `FriendFlags` bitmask; `4` is `IMMEDIATE`, which is the
    /// ordinary friends list and what almost every caller wants. It is a number
    /// rather than an enum because the underlying type is a bitmask, and
    /// bitmasks compose where enums do not.
    #[napi]
    pub fn get_friends(flags: u16) -> Vec<SteamFriend> {
        let client = crate::client::get_client();
        client
            .friends()
            .get_friends(FriendFlags::from_bits_truncate(flags))
            .into_iter()
            .map(|friend| SteamFriend {
                steam_id: PlayerSteamId::from_steamid(friend.id()),
                name: friend.name(),
                state: state_name(friend.state()),
                game_played: friend.game_played().map(|game| FriendGamePlayed {
                    game_id: BigInt::from(game.game.raw()),
                    // Steam reports "no lobby" as a zero id rather than as an
                    // absence, and a zero here is a lobby nobody can join.
                    lobby: match game.lobby.raw() {
                        0 => None,
                        id => Some(BigInt::from(id)),
                    },
                }),
            })
            .collect()
    }

    /// A friend's avatar, as raw RGBA.
    ///
    /// `size` is `small`, `medium` or `large`, defaulting to medium — which are
    /// 32x32, 64x64 and 184x184 respectively. The dimensions are implied by the
    /// size rather than returned, exactly as they are in the underlying API, so
    /// the length is always `w * h * 4`.
    ///
    /// `None` when Steam has not cached that avatar yet, which is a real state
    /// rather than an error: it arrives later, announced by a
    /// `PersonaStateChange` callback. Callers should draw something else and ask
    /// again rather than treat it as a failure.
    #[napi]
    pub fn get_avatar(steam_id64: BigInt, size: Option<String>) -> Option<Buffer> {
        let client = crate::client::get_client();
        let friend = client
            .friends()
            .get_friend(SteamId::from_raw(steam_id64.get_u64().1));
        let bytes = match size.as_deref() {
            Some("small") => friend.small_avatar(),
            Some("large") => friend.large_avatar(),
            _ => friend.medium_avatar(),
        };
        bytes.map(Buffer::from)
    }
}
