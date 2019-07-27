use derive_more::Display;

/// All possible scopes that can be requested from a user when authenticating.
///
/// Descriptions are from Reddit.
#[derive(Clone, Copy, Debug, Display, Hash, PartialEq, Eq)]
pub enum Scope {
    /// Update preferences and related account information.
    /// Will not have access to your email or password.
    #[display(fmt = "account")]
    Account,

    /// Spend my reddit gold creddits on giving gold to other users.
    #[display(fmt = "creddits")]
    Creddits,

    /// Edit and delete my comments and submissions.
    #[display(fmt = "edit")]
    Edit,

    /// Select my subreddit flair.
    /// Change link flair on my submissions.
    #[display(fmt = "flair")]
    Flair,

    /// Access my voting history and comments or submissions I've saved or hidden.
    #[display(fmt = "history")]
    History,

    /// Access my reddit username and signup date.
    #[display(fmt = "identity")]
    Identity,

    /// Manage settings and contributors of live threads I contribute to.
    #[display(fmt = "livemanage")]
    LiveManage,

    /// Manage the configuration, sidebar, and CSS of subreddits I moderate.
    #[display(fmt = "modconfig")]
    ModConfig,

    /// Add/remove users to approved submitter lists and
    /// ban/unban or mute/unmute users from subreddits I moderate.
    #[display(fmt = "modcontributors")]
    ModContributors,

    /// Manage and assign flair in subreddits I moderate.
    #[display(fmt = "modflair")]
    ModFlair,

    /// Access the moderation log in subreddits I moderate.
    #[display(fmt = "modlog")]
    ModLog,

    /// Access and manage modmail via mod.reddit.com.
    #[display(fmt = "modmail")]
    ModMail,

    /// Invite or remove other moderators from subreddits I moderate.
    #[display(fmt = "modothers")]
    ModOthers,

    /// Approve, remove, mark nsfw, and distinguish content in subreddits I moderate.
    #[display(fmt = "modposts")]
    ModPosts,

    /// Accept invitations to moderate a subreddit.
    /// Remove myself as a moderator or contributor of subreddits I moderate or contribute to.
    #[display(fmt = "modself")]
    ModSelf,

    /// Access traffic stats in subreddits I moderate.
    #[display(fmt = "modtraffic")]
    ModTraffic,

    /// Change editors and visibility of wiki pages in subreddits I moderate.
    #[display(fmt = "modwiki")]
    ModWiki,

    /// Access the list of subreddits I moderate, contribute to, and subscribe to.
    #[display(fmt = "mysubreddits")]
    MySubreddits,

    /// Access my inbox and send private messages to other users.
    #[display(fmt = "privatemessages")]
    PrivateMessages,

    /// Access posts and comments through my account.
    #[display(fmt = "read")]
    Read,

    /// Report content for rules violations. Hide & show individual submissions.
    #[display(fmt = "report")]
    Report,

    /// Save and unsave comments and submissions.
    #[display(fmt = "save")]
    Save,

    /// Edit structured styles for a subreddit I moderate.
    #[display(fmt = "structuredstyles")]
    StructuredStyles,

    /// Submit links and comments from my account.
    #[display(fmt = "submit")]
    Submit,

    /// Manage my subreddit subscriptions. Manage "friends" — users whose content I follow.
    #[display(fmt = "subscribe")]
    Subscribe,

    /// Submit and change my votes on comments and submissions.
    #[display(fmt = "vote")]
    Vote,

    /// Edit wiki pages on my behalf
    #[display(fmt = "wikiedit")]
    WikiEdit,

    /// Read wiki pages through my account
    #[display(fmt = "wikiread")]
    WikiRead,
}
