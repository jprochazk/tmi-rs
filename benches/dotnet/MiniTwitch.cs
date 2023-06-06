using System.Buffers;
using System.Collections;
using System.Diagnostics.CodeAnalysis;
using System.Text;

namespace libs_comparison;
public static class MiniTwitch
{
    public static void Process(ReadOnlyMemory<byte> data)
    {
        (IrcCommand command, int lfIndex) = ParseCommand(data.Span);
        int accumulatedIndex = lfIndex;
        ReceiveData(command, data);
        while (lfIndex != 0 && data.Length - accumulatedIndex > 0)
        {
            (command, lfIndex) = ParseCommand(data.Span[accumulatedIndex..]);
            ReceiveData(command, data[accumulatedIndex..]);
            accumulatedIndex += lfIndex;
        }
    }

    public static (IrcCommand cmd, int lfIndex) ParseCommand(ReadOnlySpan<byte> span)
    {
        const byte space = (byte)' ';
        const byte colon = (byte)':';
        const byte at = (byte)'@';
        const byte lf = (byte)'\n';

        int scopeStart;
        int firstSpace;
        int startIndex;
        int length;
        ReadOnlySpan<byte> command;

        if (span[0] == lf)
        {
            return (IrcCommand.Unknown, 0);
        }
        else if (span[0] is not colon and not at)
        {
            firstSpace = span.IndexOf(space);
            command = span[..firstSpace];
        }
        else
        {
            scopeStart = span.IndexOf(space);
            if (span[scopeStart + 1] == colon)
            {
                scopeStart += 2;
                firstSpace = span[scopeStart..].IndexOf(space) + scopeStart;
                startIndex = firstSpace + 1;
                length = span[startIndex..].IndexOf(space);

                if (firstSpace == scopeStart - 1 || length == -1)
                {
                    return (IrcCommand.Unknown, -1);
                }

                command = span[startIndex..(startIndex + length)];
            }
            else
            {
                ++scopeStart;
                int secondSpace = span[scopeStart..].IndexOf(space) + scopeStart;
                if (secondSpace == scopeStart - 1)
                {
                    secondSpace = span.Length;
                }

                command = span[scopeStart..secondSpace];
            }
        }

        return ((IrcCommand)command.Sum(), span.IndexOf(lf) + 1);
    }

    private static IrcClient _c = new();
    public static void ReceiveData(IrcCommand command, ReadOnlyMemory<byte> data)
    {
        switch (command)
        {
            case IrcCommand.PRIVMSG:
                Privmsg ircMessage = new(data, _c);
                _ = ircMessage;
                break;

            case IrcCommand.Connected:
                _ = command;
                break;

            case IrcCommand.RECONNECT:
                _ = command;
                break;

            case IrcCommand.PING:
                _ = command;
                break;

            case IrcCommand.USERNOTICE:
                Usernotice usernotice = new(data);
                switch (usernotice.MsgId)
                {
                    case UsernoticeType.Sub
                    or UsernoticeType.Resub:
                        _ = usernotice;
                        break;

                    case UsernoticeType.Subgift:
                        _ = usernotice;
                        break;

                    case UsernoticeType.Raid:
                        _ = usernotice;
                        break;

                    case UsernoticeType.AnonGiftPaidUpgrade
                    or UsernoticeType.GiftPaidUpgrade:
                        _ = usernotice;
                        break;

                    case UsernoticeType.PrimePaidUpgrade:
                        _ = usernotice;
                        break;

                    case UsernoticeType.Announcement:
                        _ = usernotice;
                        break;

                    case UsernoticeType.SubMysteryGift:
                        _ = usernotice;
                        break;
                }

                break;

            case IrcCommand.CLEARCHAT:
                Clearchat clearchat = new(data);
                if (clearchat.IsClearChat)
                {
                    _ = clearchat;
                }
                else
                {
                    _ = clearchat.IsBan ? clearchat : clearchat;
                }

                break;

            case IrcCommand.CLEARMSG:
                Clearmsg clearmsg = new(data);
                _ = clearmsg;
                break;

            case IrcCommand.ROOMSTATE:
                IrcChannel ircChannel = new(data);
                if (ircChannel.Roomstate == RoomstateType.All)
                {
                    _ = ircChannel;
                }
                else if (ircChannel.Roomstate == RoomstateType.EmoteOnly)
                {
                    _ = ircChannel;
                }
                else if (ircChannel.Roomstate == RoomstateType.FollowerOnly)
                {
                    _ = ircChannel;
                }
                else if (ircChannel.Roomstate == RoomstateType.R9K)
                {
                    _ = ircChannel;
                }
                else if (ircChannel.Roomstate == RoomstateType.Slow)
                {
                    _ = ircChannel;
                }
                else
                {
                    _ = ircChannel.Roomstate == RoomstateType.SubOnly ? ircChannel : ircChannel;
                }

                break;

            case IrcCommand.PART:
                IrcChannel channel = new(data);
                _ = channel;
                break;

            case IrcCommand.NOTICE:
                Notice notice = new(data);
                if (notice.Type == NoticeType.Msg_channel_suspended)
                {
                    _ = notice;
                }
                else if (notice.Type == NoticeType.Bad_auth)
                {
                    _ = notice;
                }

                _ = notice;
                break;

            case IrcCommand.USERSTATE or IrcCommand.GLOBALUSERSTATE:
                Userstate state = new(data);
                if (state.Self.IsMod)
                {
                    _ = state;
                }

                _ = state;
                break;

            case IrcCommand.WHISPER:
                Whisper whisper = new(data);
                _ = whisper;
                break;
        }
    }

    public static int Sum(this ReadOnlySpan<byte> source)
    {
        int sum = 0;
        foreach (byte b in source)
        {
            sum += b;
        }

        return sum;
    }

    public static string FindChannel(this ReadOnlySpan<byte> span, bool anySeparator = false)
    {
        const byte numberSymbol = (byte)'#';
        const byte space = (byte)' ';
        const byte lf = (byte)'\n';
        const byte cr = (byte)'\r';

        int symbolIndex = span.IndexOf(numberSymbol);
        int secondSymbolIndex = span[(symbolIndex + 1)..].IndexOf(numberSymbol) + symbolIndex + 1;
        int nextSpace;
        if (anySeparator)
        {
            ReadOnlySpan<byte> ends = stackalloc byte[] { space, lf, cr };
            nextSpace = span[secondSymbolIndex..].IndexOfAny(ends) + secondSymbolIndex;
        }
        else
        {
            nextSpace = span[secondSymbolIndex..].IndexOf(space) + secondSymbolIndex;
        }

        if (nextSpace == secondSymbolIndex - 1)
        {
            nextSpace = span.Length;
        }

        ReadOnlySpan<byte> newSpan = span[(secondSymbolIndex + 1)..nextSpace];
        return TagHelper.GetString(newSpan, true);
    }

    public static (string Content, bool Action) FindContent(this ReadOnlySpan<byte> span, bool maybeEmpty = false, bool maybeAction = false)
    {
        const byte colon = (byte)':';
        const byte space = (byte)' ';

        int firstSeparatorIndex = span.IndexOf(space);
        int secondSeparatorIndex = span[(firstSeparatorIndex + 2)..].IndexOf(colon) + firstSeparatorIndex + 2;
        if (maybeEmpty && secondSeparatorIndex - firstSeparatorIndex - 2 == -1)
        {
            return (string.Empty, false);
        }

        ReadOnlySpan<byte> newSpan = span[(secondSeparatorIndex + 1)..];
        string content = TagHelper.GetString(newSpan);
        if (maybeAction && content.Length >= 7 && content[0] == '\u0001' && content[^1] == '\u0001')
        {
            return (content[9..^1], true);
        }

        return (content, false);
    }

    public static string FindUsername(this ReadOnlySpan<byte> span)
    {
        const byte space = (byte)' ';
        const byte exclamationMark = (byte)'!';

        int separator = span.IndexOf(space);
        int exclamationIndex = span[separator..].IndexOf(exclamationMark) + separator;

        ReadOnlySpan<byte> newSpan = span[(separator + 2)..exclamationIndex];
        return TagHelper.GetString(newSpan);
    }

    public static IrcTags ParseTags(ReadOnlyMemory<byte> memory)
    {
        ReadOnlySpan<byte> span = memory.Span;

        const byte at = (byte)'@';
        const byte space = (byte)' ';
        const byte colon = (byte)':';
        const byte semiColon = (byte)';';
        const byte equals = (byte)'=';

        int tagsStartIndex = span[0] is at or colon ? 1 : 0;
        int tagsEndIndex = span.IndexOf(space);
        int tagCount = 0;

        int eqIndex;
        ReadOnlySpan<byte> tagIndexCountSpan = span;
        while ((eqIndex = tagIndexCountSpan.IndexOf(equals)) != -1)
        {
            tagCount++;
            tagIndexCountSpan = tagIndexCountSpan[(eqIndex + 1)..];
        }

        IrcTags tags = new(tagCount);
        int tagStart = tagsStartIndex;
        int tagEquals;
        int tagEnd;

        for (int i = 0; i < tagCount; i++)
        {
            if (tagStart >= tagsEndIndex)
            {
                break;
            }

            tagEquals = span[tagStart..tagsEndIndex].IndexOf(equals) + tagStart;
            if (tagEquals == tagStart - 1)
            {
                break;
            }

            tagEnd = span[tagEquals..tagsEndIndex].IndexOf(semiColon) + tagEquals;
            if (tagEnd == tagEquals - 1)
            {
                tagEnd = tagsEndIndex;
            }

            tags.Add(i, memory[tagStart..tagEquals], memory[(tagEquals + 1)..tagEnd]);

            tagStart = tagEnd + 1;
        }

        return tags;
    }
}

public enum IrcCommand
{
    [Obsolete("Unused")] Connected_host = 146,
    [Obsolete("Unused")] Connected_motd = 147,
    [Obsolete("Unused")] Connected_useless = 148,
    [Obsolete("Unused")] NamesList = 155,
    [Obsolete("Unused")] Connected_useless_motd = 156,
    [Obsolete("Unused")] Connected_useless2_or_NamesListEnd_who_cares = 159,
    [Obsolete("Unused")] Connected_end_of_bullshit = 160,

    Unknown = 0,
    Connected = 145,
    Capabilities_received = 212,
    PING = 302,
    JOIN = 304,
    PONG = 308,
    PART = 311,
    NOTICE = 450,
    WHISPER = 546,
    PRIVMSG = 552,
    CLEARMSG = 590,
    CLEARCHAT = 647,
    RECONNECT = 673,
    ROOMSTATE = 702,
    USERSTATE = 704,
    USERNOTICE = 769,
    GLOBALUSERSTATE = 1137
}

public readonly struct Privmsg : IUnixTimestamped, IEquatable<Privmsg>
{
    /// <summary>
    /// Author of the message
    /// </summary>
    public MessageAuthor Author { get; }
    /// <summary>
    /// Reply contents of the message
    /// <para>Note: Values are <see cref="string.Empty"/> if the <see cref="MessageReply.HasContent"/> is <see langword="false"/></para>
    /// </summary>
    public MessageReply Reply { get; init; }
    /// <summary>
    /// The channel where the message was sent
    /// </summary>
    public IBasicChannel Channel { get; init; }
    /// <summary>
    /// Content of the message
    /// </summary>
    public string Content { get; init; }
    /// <summary>
    /// Emote sets in the content of the message
    /// <para><see cref="string.Empty"/> if there are none</para>
    /// </summary>
    public string Emotes { get; init; }
    /// <summary>
    /// Automod flags in the content of the message
    /// <para><see cref="string.Empty"/> if there are none</para>
    /// </summary>
    public string Flags { get; init; }
    /// <summary>
    /// Unique ID to identify the message
    /// </summary>
    public string Id { get; init; }
    /// <summary>
    /// The amount of bits cheered in the message
    /// <para>Default is 0</para>
    /// </summary>
    public int Bits { get; init; }
    /// <summary>
    /// Client nonce that was sent with the message
    /// <para>Note: Can be <see cref="string.Empty"/></para>
    /// </summary>
    public string Nonce { get; init; }
    /// <summary>
    /// Whether the was the author's first message in the channel or not
    /// </summary>
    public bool IsFirstMessage { get; init; }
    /// <summary>
    /// Whether the message as an action or not. Action messages are sent with .me
    /// </summary>
    public bool IsAction { get; init; }
    /// <summary>
    /// Whether the user is a returning chatter or not
    /// <para>This tag is not documented <see href="https://dev.twitch.tv/docs/irc/tags/#privmsg-tags"/> </para>
    /// </summary>
    public bool IsReturningChatter { get; init; }

    /// <inheritdoc/>
    public long TmiSentTs { get; init; }
    /// <inheritdoc/>
    public DateTimeOffset SentTimestamp => DateTimeOffset.FromUnixTimeMilliseconds(this.TmiSentTs);

    public IrcClient? Source { get; init; }

    public Privmsg(ReadOnlyMemory<byte> memory, IrcClient? source = null)
    {
        this.Source = source;

        // MessageAuthor
        string badges = string.Empty;
        string badgeInfo = string.Empty;
        string color = string.Empty;
        string displayName = string.Empty;
        string username = memory.Span.FindUsername();
        long uid = 0;
        bool mod = false;
        bool sub = false;
        bool turbo = false;
        bool vip = false;
        UserType userType = UserType.None;

        // MessageReply
        bool hasReply = false;
        string replyMessageId = string.Empty;
        long replyUserId = 0;
        string replyMessageBody = string.Empty;
        string replyUsername = string.Empty;
        string replyDisplayName = string.Empty;

        // IBasicChannel
        string channelName = memory.Span.FindChannel();
        long channelId = 0;

        (this.Content, this.IsAction) = memory.Span.FindContent(maybeAction: true);

        string emotes = string.Empty;
        string flags = string.Empty;
        string id = string.Empty;
        int bits = 0;
        string nonce = string.Empty;
        long tmiSentTs = 0;
        bool firstMsg = false;
        bool returningChatter = false;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;

            switch (tagKey.Sum())
            {
                //id
                case 205:
                    id = TagHelper.GetString(tagValue);
                    break;

                //mod
                case 320:
                    mod = TagHelper.GetBool(tagValue);
                    break;

                //vip
                case 335:
                    vip = TagHelper.GetBool(tagValue);
                    break;

                //bits
                case 434:
                    bits = TagHelper.GetInt(tagValue);
                    break;

                //flags
                case 525:
                    flags = TagHelper.GetString(tagValue);
                    break;

                //color
                case 543:
                    color = TagHelper.GetString(tagValue, true);
                    break;

                //turbo
                case 556:
                    turbo = TagHelper.GetBool(tagValue);
                    break;

                //badges
                case 614:
                    badges = TagHelper.GetString(tagValue, true);
                    break;

                //emotes
                case 653:
                    emotes = TagHelper.GetString(tagValue);
                    break;

                //room-id
                case 695:
                    channelId = TagHelper.GetLong(tagValue);
                    break;

                //user-id
                case 697:
                    uid = TagHelper.GetLong(tagValue);
                    break;

                //first-msg
                case 924:
                    firstMsg = TagHelper.GetBool(tagValue);
                    break;

                //user-type
                case 942 when tagValue.Length > 0:
                    userType = TagHelper.GetEnum<UserType>(tagValue);
                    break;

                //badge-info
                case 972:
                    badgeInfo = TagHelper.GetString(tagValue, true);
                    break;

                //subscriber
                case 1076:
                    sub = TagHelper.GetBool(tagValue);
                    break;

                //tmi-sent-ts
                case 1093:
                    tmiSentTs = TagHelper.GetLong(tagValue);
                    break;

                //client-nonce
                case 1215:
                    nonce = TagHelper.GetString(tagValue);
                    break;

                //display-name
                case 1220:
                    displayName = TagHelper.GetString(tagValue);
                    break;

                //returning-chatter
                case 1782:
                    returningChatter = TagHelper.GetBool(tagValue);
                    break;

                //reply-parent-msg-id
                case 1873:
                    replyMessageId = TagHelper.GetString(tagValue);
                    hasReply = true;
                    break;

                //reply-parent-user-id
                case 1993:
                    replyUserId = TagHelper.GetLong(tagValue);
                    break;

                //reply-parent-msg-body
                case 2098:
                    replyMessageBody = TagHelper.GetString(tagValue, unescape: true);
                    break;

                //reply-parent-user-login
                case 2325:
                    replyUsername = TagHelper.GetString(tagValue);
                    break;

                //reply-parent-display-name
                case 2516:
                    replyDisplayName = TagHelper.GetString(tagValue);
                    break;
            }
        }

        this.Author = new MessageAuthor()
        {
            BadgeInfo = badgeInfo,
            Badges = badges,
            ColorCode = color,
            DisplayName = displayName,
            Id = uid,
            IsMod = mod,
            IsSubscriber = sub,
            Type = userType,
            Name = username,
            IsTurbo = turbo,
            IsVip = vip
        };
        this.Reply = new MessageReply()
        {
            ParentMessageId = replyMessageId,
            ParentDisplayName = replyDisplayName,
            ParentMessage = replyMessageBody,
            ParentUserId = replyUserId,
            ParentUsername = replyUsername,
            HasContent = hasReply
        };
        this.Channel = new IrcChannel()
        {
            Name = channelName,
            Id = channelId
        };
        this.Emotes = emotes;
        this.Flags = flags;
        this.Id = id;
        this.Bits = bits;
        this.Nonce = nonce;
        this.TmiSentTs = tmiSentTs;
        this.IsFirstMessage = firstMsg;
        this.IsReturningChatter = returningChatter;
    }

    /// <summary>
    /// Reply to the message
    /// </summary>
    /// <param name="reply">The reply to send</param>
    /// <param name="action">Prepend .me</param>
    /// <returns></returns>
    public ValueTask ReplyWith(string reply, bool action = false) => this.Source?.ReplyTo(this, reply, action) ?? ValueTask.CompletedTask;

    /// <summary>
    /// Construct a message from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC message <para>Example input: @badge-info=subscriber/10;badges=subscriber/6;color=#F2647B;display-name=occluder;emotes=;first-msg=0;flags=;id=1eef01e3-634a-493b-b1a7-4f65040fa986;mod=0;returning-chatter=0;room-id=11148817;subscriber=1;tmi-sent-ts=1679231590118;turbo=0;user-id=783267696;user-type= :occluder!occluder@occluder.tmi.twitch.tv PRIVMSG #pajlada :-tags lol!</para></param>
    /// <returns><see cref="Privmsg"/> with the related data</returns>
    public static Privmsg Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }

    /// <inheritdoc/>
    public bool Equals(Privmsg other) => other.Id == this.Id;

    /// <inheritdoc/>
    public bool Equals(string? other) => this.Content.Equals(other);

    // Don't remove this, compiler wont shut up
#pragma warning disable CS8765
    /// <inheritdoc/>
    public override bool Equals([NotNull] object obj) => obj is Privmsg && Equals((Privmsg)obj);
#pragma warning restore CS8765
    /// <inheritdoc/>
    public static bool operator ==(Privmsg left, Privmsg right) => left.Equals(right);
    /// <inheritdoc/>
    public static bool operator !=(Privmsg left, Privmsg right) => !(left == right);
    /// <inheritdoc/>
    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(this.Id);
        hash.Add(this.Content);
        return hash.ToHashCode();
    }

    /// <inheritdoc/>
    public static implicit operator string(Privmsg message) => message.Content;
}

public static class TagHelper
{
    public static string GetString(ReadOnlySpan<byte> span, bool intern = false, bool unescape = false)
    {
        string value;

        if (unescape)
        {
            Span<byte> unescaped = stackalloc byte[span.Length];
            Unescape(span, unescaped);
            value = Encoding.UTF8.GetString(unescaped);
        }
        else
        {
            value = Encoding.UTF8.GetString(span);
        }

        if (intern)
        {
            return string.IsInterned(value) ?? string.Intern(value);
        }

        return value;
    }

    public static bool GetBool(ReadOnlySpan<byte> span, bool nonBinary = false)
    {
        const byte zero = (byte)'0';
        if (nonBinary)
        {
            string value = Encoding.UTF8.GetString(span);
            string interned = string.IsInterned(value) ?? string.Intern(value);

            return bool.Parse(interned);
        }

        return span[0] != zero;
    }

    public static int GetInt(ReadOnlySpan<byte> span)
    {
        const byte dash = (byte)'-';
        return span[0] == dash ? -1 * ParseInt(span[1..]) : ParseInt(span);
    }

    public static long GetLong(ReadOnlySpan<byte> span)
    {
        const byte dash = (byte)'-';
        return span[0] == dash ? -1 * ParseLong(span[1..]) : ParseLong(span);
    }

    public static TEnum GetEnum<TEnum>(ReadOnlySpan<byte> span)
    where TEnum : struct
    {
        string value = Encoding.UTF8.GetString(span);
        string interned = string.IsInterned(value) ?? string.Intern(value);

        return Enum.Parse<TEnum>(interned, true);
    }

    private static void Unescape(ReadOnlySpan<byte> source, Span<byte> destination)
    {
        const byte backSlash = (byte)'\\';

        const byte s = (byte)'s';
        const byte colon = (byte)':';
        const byte r = (byte)'r';
        const byte n = (byte)'n';

        const byte space = (byte)' ';
        const byte semicolon = (byte)';';
        const byte cr = (byte)'\r';
        const byte lf = (byte)'\n';

        source.CopyTo(destination);
        if (source.IndexOf(backSlash) == -1)
        {
            return;
        }

        int atIndex = 0;
        int slashIndex;
        while ((slashIndex = source[atIndex..].IndexOf(backSlash)) != -1)
        {
            destination[atIndex + slashIndex] = source[atIndex + slashIndex + 1] switch
            {
                s => space,
                colon => semicolon,
                r => cr,
                n => lf,
                _ => backSlash
            };
            destination[atIndex + slashIndex + 1] = 0;

            atIndex += slashIndex + 2;
        }
    }

    private static int ParseInt(ReadOnlySpan<byte> span)
    {
        const byte numBase = (byte)'0';

        int result = 0;
        foreach (byte b in span)
        {
            result *= 10;
            result += b - numBase;
        }

        return result;
    }

    private static long ParseLong(ReadOnlySpan<byte> span)
    {
        const byte numBase = (byte)'0';

        long result = 0;
        foreach (byte b in span)
        {
            result *= 10L;
            result += b - numBase;
        }

        return result;
    }
}

public readonly struct MessageAuthor : IBanTarget, IDeletedMessageAuthor, IWhisperAuthor,
    IGiftSubRecipient, IUserstateSelf
{
    /// <summary>
    /// Contains metadata related to the chat badges in the badges tag
    /// <para>Currently, this tag contains metadata only for subscriber badges, to indicate the number of months the user has been a subscriber</para>
    /// </summary>
    public string BadgeInfo { get; init; }
    /// <summary>
    /// Comma-separated list of chat badges in the form, &lt;badge&gt;/&lt;version&gt;. For example, admin/1. There are many possible badge values, but here are few:
    /// <list type="bullet">
    /// <item>admin</item>
    /// <item>bits</item>
    /// <item>broadcaster</item>
    /// <item>moderator</item>
    /// <item>subscriber</item>
    /// <item>turbo</item>
    /// </list>
    /// <para>Most badges have only 1 version, but some badges like subscriber badges offer different versions of the badge depending on how long the user has subscribed</para>
    /// <para>To get the badge, use the <see href="https://dev.twitch.tv/docs/api/reference#get-global-chat-badges">Get Global Chat Badges</see> and <see href="https://dev.twitch.tv/docs/api/reference#get-channel-chat-badges">Get Channel Chat Badges</see> APIs. Match the badge to the <c>set-id</c> field’s value in the response. Then, match the version to the <c>id</c> field in the list of versions</para>
    /// </summary>
    public string Badges { get; init; }
    /// <summary>
    /// The color of the user’s name in the chat room. This is a hexadecimal RGB color code in the form, #RGB
    /// <para>Note: May be empty if it is never set</para>
    /// </summary>
    public string ColorCode { get; init; }
    /// <summary>
    /// The user’s display name, escaped as described in the <see href="https://ircv3.net/specs/core/message-tags-3.2.html">IRCv3</see> spec
    /// <para>Note: Can contain characters outside [a-zA-Z0-9_]</para>
    /// </summary>
    public string DisplayName { get; init; }
    /// <summary>
    /// The user's name
    /// </summary>
    public string Name { get; init; }
    /// <summary>
    /// The user's ID
    /// </summary>
    public long Id { get; init; }
    /// <summary>
    /// The type of the user 
    /// </summary>
    public UserType Type { get; init; }
    /// <summary>
    /// whether the user is a subscriber
    /// </summary>
    public bool IsSubscriber { get; init; }
    /// <summary>
    /// Whether the user is a moderator
    /// </summary>
    public bool IsMod { get; init; }
    /// <summary>
    /// Whether the user is a VIP
    /// </summary>
    public bool IsVip { get; init; }
    /// <summary>
    /// Whether the user has site-wide commercial free mode enabled
    /// <para>Note: This value is always <see langword="false"/></para>
    /// </summary>
    [Obsolete("Always false")]
    public bool IsTurbo { get; init; }

    /// <inheritdoc/>
    public static implicit operator string(MessageAuthor messageAuthor) => messageAuthor.Name;
    /// <inheritdoc/>
    public static implicit operator long(MessageAuthor messageAuthor) => messageAuthor.Id;
}

public interface IBanTarget
{
    /// <summary>
    /// Username of the user receiving the ban
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the user receiving the ban
    /// </summary>
    long Id { get; }
}

public interface IUserBan
{
    /// <summary>
    /// The target user of the ban
    /// </summary>
    IBanTarget Target { get; }
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    IBasicChannel Channel { get; }
    /// <summary>
    /// Milliseconds Unix timestamp of when the event occurred
    /// </summary>
    long TmiSentTs { get; }
}

public interface IUserstateSelf
{
    /// <inheritdoc cref="MessageAuthor.BadgeInfo"/>
    string BadgeInfo { get; }
    /// <inheritdoc cref="MessageAuthor.Badges"/>
    string Badges { get; }
    /// <inheritdoc cref="MessageAuthor.ColorCode"/>
    string ColorCode { get; }
    /// <summary>
    /// Your username
    /// </summary>
    string Name { get; }
    /// <summary>
    /// Your display name
    /// </summary>
    string DisplayName { get; }
    /// <summary>
    /// Your user type
    /// </summary>
    UserType Type { get; }
    /// <summary>
    /// Whether you are a moderator
    /// </summary>
    bool IsMod { get; }
    /// <summary>
    /// Whether you are a VIP
    /// </summary>
    bool IsVip { get; }
    /// <summary>
    /// Whether you are a subscriber
    /// </summary>
    bool IsSubscriber { get; }
    /// <summary>
    /// Whether you have site-wide commercial free mode enabled
    /// <para>Note: This value is always <see langword="false"/></para>
    /// </summary>
    [Obsolete("Always false")]
    bool IsTurbo { get; }
}

public interface IUserTimeout
{
    /// <summary>
    /// The duration of the timeout
    /// </summary>
    TimeSpan Duration { get; }
    /// <summary>
    /// The target user of the timeout
    /// </summary>
    IBanTarget Target { get; }
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    IBasicChannel Channel { get; }
    /// <summary>
    /// Milliseconds Unix timestamp of when the event occurred
    /// </summary>
    long TmiSentTs { get; }
}

public interface IBasicChannel
{
    /// <summary>
    /// The channel's username
    /// </summary>
    string Name { get; }
    /// <summary>
    /// The channel's ID
    /// </summary>
    long Id { get; }
}

public enum UserType
{
    None,
    Mod,
    Staff,
    GlobalModerator,
    Admin
}

public enum SubPlan
{
    None,
    Prime,
    Tier1,
    Tier2,
    Tier3
}

public enum NoticeType
{
    Unknown,
    /// <summary>
    /// Indicates that "Emote Only" mode has been enabled.
    /// </summary>
    Emote_only_on,
    /// <summary>
    /// Indicates that "Emote Only" mode has been disabled.
    /// </summary>
    Emote_only_off,
    /// <summary>
    /// Indicates that "Followers Only" mode has been enabled.
    /// <para>Note: Unlike <see cref="Followers_on_zero"/>, this notice is given when a user needs to be following for X amount of minutes</para>
    /// </summary>
    Followers_on,
    /// <summary>
    /// Indicates that "Followers Only" mode has been enabled.
    /// </summary>
    Followers_on_zero,
    /// <summary>
    /// Indicates that "Followers Only" mode has been disabled.
    /// </summary>
    Followers_off,
    /// <summary>
    /// Indicates that "Subs Only" mode has been enabled.
    /// </summary>
    Subs_on,
    /// <summary>
    /// Indicates that "Subs Only" mode has been disabled.
    /// </summary>
    Subs_off,
    /// <summary>
    /// Indicates that "Unique" mode has been enabled.
    /// </summary>
    R9K_on,
    /// <summary>
    /// Indicates that "Unique" mode has been disabled.
    /// </summary>
    R9K_off,
    /// <summary>
    /// Indicates that "Slow" mode has been enabled.
    /// </summary>
    Slow_on,
    /// <summary>
    /// Indicates that "Slow" mode has been disabled.
    /// </summary>
    Slow_off,
    /// <summary>
    /// The response to a "/help" message
    /// </summary>
    Cmds_available,
    /// <summary>
    /// Indicates that you have tried joining a suspended channel
    /// </summary>
    Msg_channel_suspended,
    /// <summary>
    /// Indicates that you have tried sending a duplicate message
    /// <para>"Your message is identical to the one you sent within the last 30 seconds."</para>
    /// </summary>
    Msg_duplicate,
    Msg_emoteonly,
    Msg_followersonly_zero,
    Msg_followersonly,
    Msg_rejected_mandatory,
    Msg_R9K,
    Msg_slowmode,
    Msg_subsonly,
    Msg_timedout,
    Msg_banned,
    Msg_requires_verified_phone_number,
    Msg_ratelimit,
    Msg_suspended,
    Msg_verified_email,
    Raid_error_too_many_viewers,
    Raid_error_unexpected,
    Unraid_error_unexpected,
    No_permission,
    Unavailable_command,
    Invalid_user,
    Unrecognized_cmd,
    Bad_auth
}

public enum AnnouncementColor
{
    Unknown,
    Primary,
    Blue,
    Green,
    Orange,
    Purple
}

public interface IChatClear : IUnixTimestamped
{
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    IBasicChannel Channel { get; }
}

public interface IDeletedMessageAuthor
{
    /// <summary>
    /// Username of the deleted message's sender
    /// </summary>
    string Name { get; }
}

public interface IEmoteOnlyModified
{
    /// <summary>
    /// Name of the channel where the event occurred
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the channel where the event occurred
    /// </summary>
    long Id { get; }
    /// <summary>
    /// <see langword="true"/> if emote-only mode is activated; <see langword="false"/> if deactivated
    /// </summary>
    bool EmoteOnlyEnabled { get; }
}

public interface IFollowersOnlyModified
{
    /// <summary>
    /// Minimum amount of time a user needs to be following in order to chat
    /// </summary>
    TimeSpan FollowerModeDuration { get; }
    /// <summary>
    /// Name of the channel where this event occurred
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the channel where this event occurred
    /// </summary>
    long Id { get; }
    /// <summary>
    /// <see langword="true"/> if followers-only mode is activated; <see langword="false"/> if deactivated
    /// </summary>
    bool FollowerModeEnabled { get; }
}

public interface IGazatuChannel
{
    /// <summary>
    /// The channel's username
    /// </summary>
    string Name { get; }
}

public interface IGiftSubNotice : IUsernotice
{
    /// <summary>
    /// The message emitted in chat when the user gifted the subscription
    /// <para>Example: Goop_456789 gifted a Tier 1 sub to Zackpanjang! They have given 11 Gift Subs in the channel!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// The recipient of the gift subscription
    /// </summary>
    IGiftSubRecipient Recipient { get; }
    /// <summary>
    /// Name of the subscription plan
    /// <para>Example 1: Channel Subscription (mandeow)</para>
    /// <para>Example 2: Channel Subscription (forsenlol)</para>
    /// </summary>
    string SubPlanName { get; }
    /// <summary>
    /// The cumulative amount of months the recipient has been subscribed
    /// </summary>
    int Months { get; }
    /// <summary>
    /// The amount of months the recipient received in the gift subscription
    /// </summary>
    int GiftedMonths { get; }
    /// <summary>
    /// Total amount of the gifts the author has given
    /// </summary>
    int TotalGiftCount { get; }
    /// <summary>
    /// The tier of the gift subscription
    /// </summary>
    SubPlan SubPlan { get; }
}

public interface IGiftSubNoticeIntro : IUsernotice
{
    /// <summary>
    /// The message emitted in chat when the event occurs
    /// <para>Example: xHypnoticPowerx is gifting 25 Tier 1 Subs to Mande's community! They've gifted a total of 62 in the channel!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// The amount of subscriptions the author is gifting
    /// </summary>
    int GiftCount { get; }
    /// <summary>
    /// Total amount of the gifts the author has given
    /// </summary>
    int TotalGiftCount { get; }
    /// <summary>
    /// The tier of the gift subscriptions
    /// </summary>
    SubPlan SubPlan { get; }
}

public interface IGiftSubRecipient
{
    /// <inheritdoc cref="MessageAuthor.Name"/>
    string Name { get; }
    /// <inheritdoc cref="MessageAuthor.DisplayName"/>
    string DisplayName { get; }
    /// <inheritdoc cref="MessageAuthor.Id"/>
    long Id { get; }
}

public interface IPaidUpgradeNotice : IUsernotice
{
    /// <summary>
    /// The message emitted in chat when the event occurs
    /// <para>Example: special_forces_of_russia is continuing the Gift Sub they got from potnayakatka64!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// Username of the previous subscription's gifter
    /// <para>Note: Value is <see cref="string.Empty"/> if the previous subscription's gifter was anonymous</para>
    /// </summary>
    string GifterUsername { get; }
    /// <summary>
    /// Display name of the previous subscription's gifter
    /// <para>Note: Value is <see cref="string.Empty"/> if the previous subscription's gifter was anonymous</para>
    /// </summary>
    string GifterDisplayName { get; }
}

public interface IPartedChannel
{
    /// <summary>
    /// The parted channel's username
    /// </summary>
    string Name { get; }
}

public interface IPrimeUpgradeNotice : IUsernotice
{
    /// <summary>
    /// The message emitted in chat when the event occurs
    /// <para>Example: DrDisRespexs converted from a Prime sub to a Tier 1 sub!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// The tier of the new subscription
    /// </summary>
    SubPlan SubPlan { get; }
}

public interface IR9KModified
{
    /// <summary>
    /// Name of the channel where the event occurred
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the channel where the event occurred
    /// </summary>
    long Id { get; }
    /// <summary>
    /// <see langword="true"/> if unique mode is activated; <see langword="false"/> if deactivated
    /// </summary>
    bool UniqueModeEnabled { get; }
}

public interface IRaidNotice : IUsernotice
{
    /// <summary>
    /// The user raiding the channel
    /// </summary>
    new MessageAuthor Author { get; }
    /// <summary>
    /// The message emitted in chat when the event occurs
    /// <para>Example: 1 raiders from occluder have joined!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// The amount of viewers joining from the raid
    /// </summary>
    int ViewerCount { get; }
}

public interface ISlowModeModified
{
    /// <summary>
    /// The amount of time a user needs to wait between messages
    /// </summary>
    TimeSpan SlowModeDuration { get; }
    /// <summary>
    /// Username of the channel where the event occurred
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the channel where the event occurred
    /// </summary>
    long Id { get; }
    /// <summary>
    /// <see langword="true"/> if slow mode is activated; <see langword="false"/> if deactivated
    /// </summary>
    bool SlowModeEnabled { get; }
}

public interface ISubOnlyModified
{
    /// <summary>
    /// Username of the channel where the event occurred
    /// </summary>
    string Name { get; }
    /// <summary>
    /// ID of the channel where the event occurred
    /// </summary>
    long Id { get; }
    /// <summary>
    /// <see langword="true"/> if sub-only mode is activated; <see langword="false"/> if deactivated
    /// </summary>
    bool SubOnlyEnabled { get; }
}

public interface ISubNotice : IUsernotice
{
    /// <summary>
    /// Emote sets in the resubscription message
    /// <para><see cref="string.Empty"/> if there are none, or if the user is a first time sub</para>
    /// </summary>
    string Emotes { get; }
    /// <summary>
    /// Automod flags in the resubscription message
    /// <para><see cref="string.Empty"/> if there are none, or if the user is a first time sub</para>
    /// </summary>
    string Flags { get; }
    /// <summary>
    /// The message emitted in chat when the event occurs
    /// <para>Example 1: SleepyHeadszZ subscribed at Tier 1.</para>
    /// <para>Example 2: Syn993 subscribed at Tier 1. They've subscribed for 5 months, currently on a 4 month streak!</para>
    /// </summary>
    string SystemMessage { get; }
    /// <summary>
    /// Cumulative amount of months the user has been subscribed
    /// </summary>
    int CumulativeMonths { get; }
    /// <summary>
    /// Whether the user shared their month streak in the subscription message or not
    /// </summary>
    bool ShouldShareStreak { get; }
    /// <summary>
    /// How many months in a row the user has been subscribed
    /// <para>Note: Always 0 if <see cref="ShouldShareStreak"/> is <see langword="false"/></para>
    /// </summary>
    int MonthStreak { get; }
    /// <summary>
    /// The tier of the subscription
    /// </summary>
    SubPlan SubPlan { get; }
    /// <summary>
    /// Name of the subscription plan
    /// <para>Example 1: Channel Subscription (mandeow)</para>
    /// <para>Example 2: Channel Subscription (forsenlol)</para>
    /// </summary>
    string SubPlanName { get; }
    /// <summary>
    /// The user's resubscription message content
    /// <para>Note 1: Always <see cref="string.Empty"/> for first time subscribers</para>
    /// <para>Note 2: May be <see cref="string.Empty"/> even for resubscriptions</para>
    /// </summary>
    string Message { get; }
}

public interface IUnixTimestamped
{
    /// <summary>
    /// Milliseconds Unix timestamp of when the message was sent
    /// </summary>
    long TmiSentTs { get; }
    /// <summary>
    /// Gets TmiSentTs as <see cref="DateTimeOffset"/>
    /// </summary>
    DateTimeOffset SentTimestamp { get; }
}

public interface IUsernotice : IUnixTimestamped
{
    /// <summary>
    /// Author of the event
    /// </summary>
    MessageAuthor Author { get; }
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    IBasicChannel Channel { get; }
    /// <summary>
    /// Unique ID to identify the event's message
    /// </summary>
    string Id { get; }
}

public interface IWhisperAuthor
{
    /// <inheritdoc cref="MessageAuthor.Badges"/>
    string Badges { get; }
    /// <inheritdoc cref="MessageAuthor.ColorCode"/>
    string ColorCode { get; }
    /// <inheritdoc cref="MessageAuthor.DisplayName"/>
    string DisplayName { get; }
    /// <inheritdoc cref="MessageAuthor.Name"/>
    string Name { get; }
    /// <inheritdoc cref="MessageAuthor.Id"/>
    long Id { get; }
    /// <inheritdoc cref="MessageAuthor.Type"/>
    UserType Type { get; }
    /// <inheritdoc cref="MessageAuthor.IsTurbo"/>
    [Obsolete("This is always false")]
    bool IsTurbo { get; }
}

public readonly struct MessageReply
{
    /// <summary>
    /// Display name of the original message's author
    /// </summary>
    public string ParentDisplayName { get; init; }
    /// <summary>
    /// Content of the original message
    /// </summary>
    public string ParentMessage { get; init; }
    /// <summary>
    /// Unique ID to identify the original message
    /// </summary>
    public string ParentMessageId { get; init; }
    /// <summary>
    /// Name of the original message's author
    /// </summary>
    public string ParentUsername { get; init; }
    /// <summary>
    /// ID of the original message's author
    /// </summary>
    public long ParentUserId { get; init; }
    /// <summary>
    /// Whether there are reply contents in this message
    /// </summary>
    public bool HasContent { get; init; }

    /// <inheritdoc/>
    public static implicit operator string(MessageReply messageReply) => messageReply.HasContent ? messageReply.ParentMessage : string.Empty;
    /// <inheritdoc/>
    public static implicit operator bool(MessageReply messageReply) => messageReply.HasContent;
}

public readonly struct IrcChannel : IGazatuChannel, IPartedChannel, IBasicChannel,
    IEmoteOnlyModified, IFollowersOnlyModified, IR9KModified, ISlowModeModified, ISubOnlyModified,
    IEquatable<IrcChannel>, IEquatable<MessageAuthor>
{
    /// <inheritdoc/>
    public TimeSpan FollowerModeDuration { get; init; }
    /// <inheritdoc/>
    public TimeSpan SlowModeDuration { get; init; }
    /// <inheritdoc/>
    public string Name { get; init; }
    /// <inheritdoc/>
    public long Id { get; init; }
    /// <inheritdoc/>
    public bool EmoteOnlyEnabled { get; init; } = false;
    /// <inheritdoc/>
    public bool UniqueModeEnabled { get; init; } = false;
    /// <inheritdoc/>
    public bool SubOnlyEnabled { get; init; } = false;
    /// <inheritdoc/>
    public bool FollowerModeEnabled { get; init; } = false;
    /// <inheritdoc/>
    public bool SlowModeEnabled { get; init; } = false;

    public RoomstateType Roomstate { get; init; } = RoomstateType.Unknown;

    private static readonly TimeSpan _followersOnlyOffTimeSpan = TimeSpan.FromMinutes(-1);

    public IrcChannel(ReadOnlyMemory<byte> memory)
    {
        int followerModeDuration = -1;
        int slowModeDuration = 0;
        this.Name = memory.Span.FindChannel();
        long id = 0;

        bool emoteOnlyModified = false;
        bool uniqueModeModified = false;
        bool subModeModified = false;
        bool followerModeModified = false;
        bool slowModeModified = false;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;
            switch (tagKey.Sum())
            {
                //r9k
                case 278:
                    this.UniqueModeEnabled = TagHelper.GetBool(tagValue);
                    uniqueModeModified = true;
                    break;

                //slow
                case 453:
                    slowModeDuration = TagHelper.GetInt(tagValue);
                    slowModeModified = true;
                    break;

                //room-id
                case 695:
                    id = TagHelper.GetLong(tagValue);
                    break;

                //subs-only
                case 940:
                    this.SubOnlyEnabled = TagHelper.GetBool(tagValue);
                    subModeModified = true;
                    break;

                //emote-only
                case 1033:
                    this.EmoteOnlyEnabled = TagHelper.GetBool(tagValue);
                    emoteOnlyModified = true;
                    break;

                //followers-only
                case 1484:
                    followerModeDuration = TagHelper.GetInt(tagValue);
                    followerModeModified = true;
                    break;
            }
        }

        if (emoteOnlyModified
        && uniqueModeModified
        && subModeModified
        && followerModeModified
        && slowModeModified)
        {
            this.Roomstate = RoomstateType.All;
        }
        else if (emoteOnlyModified)
        {
            this.Roomstate = RoomstateType.EmoteOnly;
        }
        else if (uniqueModeModified)
        {
            this.Roomstate = RoomstateType.R9K;
        }
        else if (subModeModified)
        {
            this.Roomstate = RoomstateType.SubOnly;
        }
        else if (followerModeModified)
        {
            this.Roomstate = RoomstateType.FollowerOnly;
        }
        else if (slowModeModified)
        {
            this.Roomstate = RoomstateType.Slow;
        }

        this.FollowerModeEnabled = followerModeDuration != -1;
        this.FollowerModeDuration = followerModeDuration == -1 ? _followersOnlyOffTimeSpan : TimeSpan.FromMinutes(followerModeDuration);
        this.SlowModeEnabled = slowModeDuration != 0;
        this.SlowModeDuration = slowModeDuration == 0 ? TimeSpan.Zero : TimeSpan.FromSeconds(slowModeDuration);
        this.Id = id;
    }

    /// <summary>
    /// Construct a channel from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC message <para>Example input: @emote-only=0;followers-only=-1;r9k=0;room-id=783267696;slow=0;subs-only=0 :tmi.twitch.tv ROOMSTATE #occluder</para></param>
    /// <returns><see cref="IrcChannel"/> with the related data</returns>
    public static IrcChannel Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }

#pragma warning disable CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).
    /// <inheritdoc/>
    public bool Equals(IrcChannel other) => this.Name == other.Name;
    /// <inheritdoc/>
    public override bool Equals(object obj) => (obj is IrcChannel && Equals((IrcChannel)obj)) || (obj is MessageAuthor && Equals((MessageAuthor)obj));
    /// <inheritdoc/>
    public bool Equals(MessageAuthor other) => this.Name == other.Name || (this.Id != 0 && this.Id == other.Id);
#pragma warning restore CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).

    /// <inheritdoc/>
    public static bool operator ==(IrcChannel left, IrcChannel right) => left.Equals(right);
    /// <inheritdoc/>
    public static bool operator !=(IrcChannel left, IrcChannel right) => !(left == right);

    /// <inheritdoc/>
    public override int GetHashCode()
    {
        var code = new HashCode();
        code.Add(this.Name);
        code.Add(this.Id);
        return code.ToHashCode();
    }

    /// <inheritdoc/>
    public static implicit operator string(IrcChannel channel) => channel.Name;
    /// <inheritdoc/>
    public static implicit operator long(IrcChannel channel) => channel.Id;
}

public enum RoomstateType
{
    Unknown,
    All,
    EmoteOnly,
    SubOnly,
    Slow,
    R9K,
    FollowerOnly
}

public readonly record struct IrcTag(ReadOnlyMemory<byte> Key, ReadOnlyMemory<byte> Value);
public readonly struct IrcTags : IDisposable, IEnumerable
{
    public int Count { get; }
    private IrcTag[] Tags { get; }

    public IrcTags(int count)
    {
        this.Count = count;
        this.Tags = ArrayPool<IrcTag>.Shared.Rent(count);
    }

    public void Dispose() => ArrayPool<IrcTag>.Shared.Return(this.Tags, true);

    public void Add(int index, ReadOnlyMemory<byte> Key, ReadOnlyMemory<byte> Value) => this.Tags[index] = new(Key, Value);

    public IEnumerator<IrcTag> GetEnumerator()
    {
        for (int x = 0; x < this.Count; x++)
        {
            yield return this.Tags[x];
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
}

public class IrcClient
{
    public ValueTask ReplyTo(Privmsg a, string b, bool c) => ValueTask.CompletedTask;
}

public readonly struct Usernotice : IGiftSubNoticeIntro, IAnnouncementNotice, IPaidUpgradeNotice,
    ISubNotice, IGiftSubNotice, IRaidNotice, IPrimeUpgradeNotice, IEquatable<Usernotice>
{
    /// <inheritdoc/>
    public MessageAuthor Author { get; init; }
    /// <inheritdoc/>
    public IGiftSubRecipient Recipient { get; init; }
    /// <inheritdoc/>
    public IBasicChannel Channel { get; init; } = default!;
    /// <inheritdoc/>
    public SubPlan SubPlan { get; init; } = SubPlan.None;
    /// <inheritdoc/>
    public AnnouncementColor Color { get; init; } = AnnouncementColor.Unknown;
    /// <inheritdoc/>
    public string Emotes { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string Flags { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string Id { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string SubPlanName { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string SystemMessage { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string Message { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string GifterUsername { get; init; } = string.Empty;
    /// <inheritdoc/>
    public string GifterDisplayName { get; init; } = string.Empty;
    /// <inheritdoc/>
    public int CumulativeMonths { get; init; } = default;
    /// <inheritdoc/>
    public int Months { get; init; } = default;
    /// <inheritdoc/>
    public int MonthStreak { get; init; } = default;
    /// <inheritdoc/>
    public int GiftedMonths { get; init; } = default;
    /// <inheritdoc/>
    public int GiftCount { get; init; } = default;
    /// <inheritdoc/>
    public int TotalGiftCount { get; init; } = default;
    /// <inheritdoc/>
    public int ViewerCount { get; init; } = default;
    /// <inheritdoc/>
    public bool ShouldShareStreak { get; init; } = default;

    /// <inheritdoc/>
    public long TmiSentTs { get; init; } = default;
    /// <inheritdoc/>
    public DateTimeOffset SentTimestamp => DateTimeOffset.FromUnixTimeMilliseconds(this.TmiSentTs);

    public UsernoticeType MsgId { get; init; } = UsernoticeType.None;

    public Usernotice(ReadOnlyMemory<byte> memory)
    {
        long channelId = 0;
        bool isMod = false;
        string colorCode = string.Empty;
        string badges = string.Empty;
        long userId = 0;
        UserType userType = UserType.None;
        string badgeInfo = string.Empty;
        bool isSubscriber = false;
        string displayName = string.Empty;
        string username = string.Empty;
        bool isTurbo = false;

        string recipientDisplayName = string.Empty;
        string recipientUsername = string.Empty;
        long recipientId = 0;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;

            switch (tagKey.Sum())
            {
                //id
                case 205:
                    this.Id = TagHelper.GetString(tagValue);
                    break;

                //mod
                case 320:
                    isMod = TagHelper.GetBool(tagValue);
                    break;

                //login
                case 537:
                    username = TagHelper.GetString(tagValue);
                    break;

                //color
                case 543:
                    colorCode = TagHelper.GetString(tagValue, true);
                    break;

                //turbo 
                case 556:
                    isTurbo = TagHelper.GetBool(tagValue);
                    break;

                //msg-id
                case 577:
                    this.MsgId = TagHelper.GetEnum<UsernoticeType>(tagValue);
                    break;

                //badges
                case 614:
                    badges = TagHelper.GetString(tagValue, true);
                    break;

                //emotes
                case 653:
                    this.Emotes = TagHelper.GetString(tagValue);
                    break;

                //room-id
                case 695:
                    channelId = TagHelper.GetLong(tagValue);
                    break;

                //user-id
                case 697:
                    userId = TagHelper.GetLong(tagValue);
                    break;

                //user-type
                case 942 when tagValue.Length > 0:
                    userType = TagHelper.GetEnum<UserType>(tagValue);
                    break;

                //badge-info
                case 972:
                    badgeInfo = TagHelper.GetString(tagValue, true);
                    break;

                //system-msg
                case 1049:
                    this.SystemMessage = TagHelper.GetString(tagValue, unescape: true);
                    break;

                //subscriber
                case 1076:
                    isSubscriber = TagHelper.GetBool(tagValue);
                    break;

                //tmi-sent-ts
                case 1093:
                    this.TmiSentTs = TagHelper.GetLong(tagValue);
                    break;

                //display-name
                case 1220:
                    displayName = TagHelper.GetString(tagValue);
                    break;

                //msg-param-color
                case 1489:
                    this.Color = TagHelper.GetEnum<AnnouncementColor>(tagValue);
                    break;

                //msg-param-months
                case 1611:
                    this.Months = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-sub-plan
                case 1748:
                    this.SubPlan = tagValue.Sum() switch
                    {
                        193 => SubPlan.Tier1,
                        194 => SubPlan.Tier2,
                        195 => SubPlan.Tier3,
                        509 => SubPlan.Prime,
                        _ => SubPlan.None
                    };
                    break;

                //msg-param-sender-name
                case 2049:
                    this.GifterDisplayName = TagHelper.GetString(tagValue);
                    break;

                //msg-param-gift-months
                case 2082:
                    this.GiftedMonths = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-viewerCount
                case 2125:
                    this.ViewerCount = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-recipient-id
                case 2159:
                    recipientId = TagHelper.GetLong(tagValue);
                    break;

                //msg-param-sender-login
                case 2169:
                    this.GifterUsername = TagHelper.GetString(tagValue);
                    break;

                //msg-param-sender-count
                case 2185:
                    this.TotalGiftCount = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-sub-plan-name
                case 2210:
                    this.SubPlanName = TagHelper.GetString(tagValue, true, true);
                    break;

                //msg-param-streak-months
                case 2306:
                    this.MonthStreak = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-mass-gift-count
                case 2451:
                    this.GiftCount = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-cumulative-months
                case 2743:
                    this.CumulativeMonths = TagHelper.GetInt(tagValue);
                    break;

                //msg-param-recipient-user-name
                case 2863:
                    recipientUsername = TagHelper.GetString(tagValue);
                    break;

                //msg-param-should-share-streak
                case 2872:
                    this.ShouldShareStreak = TagHelper.GetBool(tagValue);
                    break;

                //msg-param-recipient-display-name
                case 3174:
                    recipientDisplayName = TagHelper.GetString(tagValue);
                    break;
            }
        }

        if (this.MsgId is UsernoticeType.Resub or UsernoticeType.Announcement)
        {
            this.Message = memory.Span.FindContent(true).Content;
        }

        this.Author = new MessageAuthor()
        {
            BadgeInfo = badgeInfo,
            Badges = badges,
            ColorCode = colorCode,
            DisplayName = displayName,
            Id = userId,
            IsMod = isMod,
            IsSubscriber = isSubscriber,
            Type = userType,
            Name = username,
            IsTurbo = isTurbo,
            IsVip = badges.Contains("vip/1")
        };
        this.Channel = new IrcChannel()
        {
            Name = memory.Span.FindChannel(),
            Id = channelId
        };
        this.Recipient = new MessageAuthor()
        {
            DisplayName = recipientDisplayName,
            Name = recipientUsername,
            Id = recipientId
        };
    }

    /// <summary>
    /// Construct a <see cref="Usernotice"/> from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC</param>
    /// <returns><see cref="Usernotice"/> with the related data</returns>
    public static Usernotice Construct(string rawData)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(rawData);
        ReadOnlyMemory<byte> memory = new(bytes);
        return new(memory);
    }

#pragma warning disable CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).
    public bool Equals(Usernotice other) => this.Id == other.Id;
    public override bool Equals(object obj) => obj is Usernotice && Equals((Usernotice)obj);
#pragma warning restore CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).

    public static bool operator ==(Usernotice left, Usernotice right) => left.Equals(right);
    public static bool operator !=(Usernotice left, Usernotice right) => !(left == right);

    public override int GetHashCode()
    {
        var code = new HashCode();
        code.Add(this.Id);
        code.Add(this.MsgId);
        return code.ToHashCode();
    }
}

public enum UsernoticeType
{
    None,
    Sub,
    Resub,
    Subgift,
    SubMysteryGift,
    GiftPaidUpgrade,
    [Obsolete("Unused")] RewardGift,
    AnonGiftPaidUpgrade,
    Raid,
    Unraid,
    [Obsolete("Unused")] Ritual,
    BitsBadgeTier,
    Announcement,
    PrimePaidUpgrade,
    StandardPayForward
}

public interface IAnnouncementNotice : IUsernotice
{
    /// <summary>
    /// Color of the announcement
    /// <para>Default is <see cref="AnnouncementColor.Primary"/></para>
    /// </summary>
    AnnouncementColor Color { get; }
    /// <summary>
    /// The message content of the announcement
    /// </summary>
    string Message { get; }
    /// <summary>
    /// Emote sets in the announcement message
    /// <para><see cref="string.Empty"/> if there are none</para>
    /// </summary>
    string Emotes { get; }
    /// <summary>
    /// Automod flags in the announcement message
    /// <para><see cref="string.Empty"/> if there are none</para>
    /// </summary>
    string Flags { get; }
}

public readonly struct Clearchat : IUserTimeout, IUserBan, IChatClear
{
    /// <summary>
    /// Duration of the timeout
    /// </summary>
    public TimeSpan Duration { get; init; }
    /// <inheritdoc/>
    public IBanTarget Target { get; init; }
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    public IBasicChannel Channel { get; init; }

    /// <inheritdoc/>
    public long TmiSentTs { get; init; }
    /// <inheritdoc/>
    public DateTimeOffset SentTimestamp => DateTimeOffset.FromUnixTimeMilliseconds(this.TmiSentTs);

    public bool IsClearChat { get; init; }
    public bool IsBan { get; init; }

    public Clearchat(ReadOnlyMemory<byte> memory)
    {
        int duration = 0;
        long targetId = 0;
        long channelId = 0;

        long tmiSentTs = 0;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;

            switch (tagKey.Sum())
            {
                //room-id
                case 695:
                    channelId = TagHelper.GetLong(tagValue);
                    break;

                //tmi-sent-ts
                case 1093:
                    tmiSentTs = TagHelper.GetLong(tagValue);
                    break;

                //ban-duration
                case 1220:
                    duration = TagHelper.GetInt(tagValue);
                    break;

                //target-user-id
                case 1389:
                    targetId = TagHelper.GetLong(tagValue);
                    break;
            }
        }

        this.Duration = duration == 0 ? TimeSpan.Zero : TimeSpan.FromSeconds(duration);
        this.Target = new MessageAuthor()
        {
            Name = memory.Span.FindContent().Content,
            Id = targetId
        };
        this.Channel = new IrcChannel()
        {
            Name = memory.Span.FindChannel(),
            Id = channelId
        };
        this.TmiSentTs = tmiSentTs;
        this.IsClearChat = targetId == 0;
        this.IsBan = duration == 0;
    }

    /// <summary>
    /// Construct a timeout or ban from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC message <para>Example input: <c></c>@badge-info=subscriber/10;badges=subscriber/6;color=#F2647B;display-name=occluder;emotes=;first-msg=0;flags=;id=5adf1e72-72b1-46c1-99df-eca4bf90120f;mod=0;returning-chatter=0;room-id=11148817;subscriber=1;tmi-sent-ts=1679785255155;turbo=0;user-id=783267696;user-type= :occluder!occluder@occluder.tmi.twitch.tv PRIVMSG #pajlada :!vanish</para></param>
    /// <returns><see cref="Clearchat"/> with the related data</returns>
    public static Clearchat Construct(string rawData)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(rawData);
        ReadOnlyMemory<byte> memory = new(bytes);
        return new(memory);
    }
}

public readonly struct Clearmsg : IUnixTimestamped
{
    /// <inheritdoc cref="IDeletedMessageAuthor"/>
    public IDeletedMessageAuthor Target { get; init; }
    /// <summary>
    /// The channel where the event occurred
    /// </summary>
    public IGazatuChannel Channel { get; init; }
    /// <summary>
    /// Unique ID identifying the deleted message
    /// </summary>
    public string MessageId { get; init; }
    /// <summary>
    /// The content of the deleted message
    /// </summary>
    public string MessageContent { get; init; }

    /// <inheritdoc/>
    public long TmiSentTs { get; init; }
    /// <inheritdoc/>
    public DateTimeOffset SentTimestamp => DateTimeOffset.FromUnixTimeMilliseconds(this.TmiSentTs);

    public Clearmsg(ReadOnlyMemory<byte> memory)
    {
        string targetUsername = string.Empty;
        string channelName = memory.Span.FindChannel();
        string messageId = string.Empty;
        long tmiSentTs = 0;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;

            switch (tagKey.Sum())
            {
                //login
                case 537:
                    targetUsername = TagHelper.GetString(tagValue);
                    break;

                //tmi-sent-ts
                case 1093:
                    tmiSentTs = TagHelper.GetLong(tagValue);
                    break;

                //target-msg-id
                case 1269:
                    messageId = TagHelper.GetString(tagValue);
                    break;

            }
        }

        this.Target = new MessageAuthor()
        {
            Name = targetUsername
        };
        this.Channel = new IrcChannel()
        {
            Name = channelName
        };
        this.MessageId = messageId;
        this.MessageContent = memory.Span.FindContent(maybeAction: true).Content;
        this.TmiSentTs = tmiSentTs;
    }

    /// <summary>
    /// Construct a "deleted message" message from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC message <para>Example input: @login=occluder;room-id=;target-msg-id=55dc74c9-a6b2-4443-9b68-3446a5ddb7ed;tmi-sent-ts=1678798254260 :tmi.twitch.tv CLEARMSG #occluder :frozen lol! </para></param>
    /// <returns><see cref="Clearmsg"/> with the related data</returns>
    public static Clearmsg Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }

    public static implicit operator string(Clearmsg clearmsg) => clearmsg.MessageContent;
}

public readonly struct Notice : IEquatable<Notice>
{
    /// <summary>
    /// The channel related to the notice
    /// </summary>
    public IGazatuChannel Channel { get; init; } = default!;
    /// <summary>
    /// The notice message
    /// </summary>
    public string SystemMessage { get; init; } = string.Empty;
    /// <summary>
    /// Type of the notice
    /// </summary>
    public NoticeType Type { get; init; } = NoticeType.Unknown;

    public Notice(ReadOnlyMemory<byte> memory)
    {
        this.SystemMessage = memory.Span.FindContent().Content;
        using IrcTags ircTags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in ircTags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;

            // JUST in case they add more shit in the future
            switch (tagKey.Sum())
            {
                //msg-id
                case 577:
                    this.Type = TagHelper.GetEnum<NoticeType>(tagValue);
                    break;
            }
        }

        try
        {
            this.Channel = new IrcChannel()
            {
                Name = memory.Span.FindChannel()
            };
        }
        catch
        {
            this.Type = NoticeType.Bad_auth;
        }
    }

    /// <summary>
    /// Construct a notice from a string. Useful for testing
    /// </summary>
    /// <param name="rawData">The raw IRC message <para>Example input: @msg-id=msg_channel_suspended :tmi.twitch.tv NOTICE #foretack :This channel does not exist or has been suspended.</para></param>
    /// <returns><see cref="Notice"/> with the related data</returns>
    public static Notice Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }

#pragma warning disable CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).
    public bool Equals(Notice other) => this.Type == other.Type && this.Channel.Name == other.Channel.Name;
    public override bool Equals(object obj) => obj is Notice && Equals((Notice)obj);
#pragma warning restore CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).

    public static bool operator ==(Notice left, Notice right) => left.Equals(right);

    public static bool operator !=(Notice left, Notice right) => !(left == right);

    public override int GetHashCode()
    {
        var code = new HashCode();
        code.Add(this.Type);
        code.Add(this.Channel.Name);
        return code.ToHashCode();
    }

    public static implicit operator string(Notice notice) => notice.SystemMessage;
    public static implicit operator NoticeType(Notice notice) => notice.Type;
}

public readonly struct Userstate
{
    /// <summary>
    /// You as a message author
    /// </summary>
    public IUserstateSelf Self { get; init; }
    /// <summary>
    /// The channel where <see cref="Self"/> applies
    /// </summary>
    public IGazatuChannel Channel { get; init; }
    /// <summary>
    /// The emote sets you have
    /// </summary>
    public string EmoteSets { get; init; }

    public IrcClient? Source { get; init; }

    public Userstate(ReadOnlyMemory<byte> memory, IrcClient? source = null)
    {
        this.Source = source;

        string badgeInfo = string.Empty;
        string badges = string.Empty;
        string color = string.Empty;
        string displayName = string.Empty;
        bool mod = false;
        bool vip = false;
        bool subscriber = false;
        bool turbo = false;
        UserType type = UserType.None;
        string channel = memory.Span.FindChannel(true);
        string emoteSets = string.Empty;

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            if (tag.Key.Length == 0)
            {
                continue;
            }

            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;
            switch (tagKey.Sum())
            {

                //mod
                case 320:
                    mod = TagHelper.GetBool(tagValue);
                    break;

                //vip
                case 335:
                    vip = TagHelper.GetBool(tagValue);
                    break;

                //color
                case 543:
                    color = TagHelper.GetString(tagValue, true);
                    break;

                //turbo
                case 556:
                    turbo = TagHelper.GetBool(tagValue);
                    break;

                //badges
                case 614:
                    badges = TagHelper.GetString(tagValue, true);
                    break;

                //user-type
                case 942 when tagValue.Length > 0:
                    type = TagHelper.GetEnum<UserType>(tagValue);
                    break;

                //badge-info
                case 972:
                    badgeInfo = TagHelper.GetString(tagValue, true);
                    break;

                //emote-sets
                case 1030:
                    emoteSets = TagHelper.GetString(tagValue, true);
                    break;

                //subscriber
                case 1076:
                    subscriber = TagHelper.GetBool(tagValue);
                    break;

                //display-name
                case 1220:
                    displayName = TagHelper.GetString(tagValue);
                    break;
            }
        }

        this.Self = new MessageAuthor()
        {
            BadgeInfo = badgeInfo,
            ColorCode = color,
            Badges = badges,
            Name = displayName.ToLower(),
            DisplayName = displayName,
            IsMod = mod || badges.Contains("broadcaster/1"),
            IsSubscriber = subscriber,
            IsTurbo = turbo,
            IsVip = vip || badges.Contains("vip/1"),
            Type = type
        };
        this.Channel = new IrcChannel()
        {
            Name = channel
        };
        this.EmoteSets = emoteSets;
    }

    public static Userstate Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }
}

public readonly struct Whisper
{
    public IWhisperAuthor Author { get; init; }
    public string Emotes { get; init; }
    public int Id { get; init; }
    public string ThreadId { get; init; }
    public string Content { get; init; }
    public bool IsAction { get; init; }

    public Whisper(ReadOnlyMemory<byte> memory)
    {
        Console.WriteLine(Encoding.UTF8.GetString(memory.Span));
        string badges = string.Empty;
        string color = string.Empty;
        string displayName = string.Empty;
        string username = memory.Span.FindUsername();
        long uid = 0;
        UserType type = UserType.None;
        bool turbo = false;

        string emotes = string.Empty;
        int id = 0;
        string threadId = string.Empty;
        (string content, bool action) = memory.Span.FindContent(maybeAction: true);

        using IrcTags tags = MiniTwitch.ParseTags(memory);
        foreach (IrcTag tag in tags)
        {
            ReadOnlySpan<byte> tagKey = tag.Key.Span;
            ReadOnlySpan<byte> tagValue = tag.Value.Span;
            switch (tagKey.Sum())
            {
                //color
                case 543:
                    color = TagHelper.GetString(tagValue, true);
                    break;

                //turbo
                case 556:
                    turbo = TagHelper.GetBool(tagValue);
                    break;

                //badges
                case 614:
                    badges = TagHelper.GetString(tagValue, true);
                    break;

                //emotes
                case 653:
                    emotes = TagHelper.GetString(tagValue);
                    break;

                //user-id
                case 697:
                    uid = TagHelper.GetLong(tagValue);
                    break;

                //thread-id
                case 882:
                    threadId = TagHelper.GetString(tagValue, true);
                    break;

                //user-type
                case 942 when tagValue.Length > 0:
                    type = TagHelper.GetEnum<UserType>(tagValue);
                    break;

                //message-id
                case 991:
                    id = TagHelper.GetInt(tagValue);
                    break;

                //display-name
                case 1220:
                    displayName = TagHelper.GetString(tagValue);
                    break;
            }
        }

        this.Author = new MessageAuthor()
        {
            Badges = badges,
            ColorCode = color,
            DisplayName = displayName,
            Name = username,
            Id = uid,
            Type = type,
            IsTurbo = turbo
        };
        this.Emotes = emotes;
        this.Id = id;
        this.ThreadId = threadId;
        this.Content = content;
        this.IsAction = action;
    }

    public static Whisper Construct(string rawData)
    {
        ReadOnlyMemory<byte> memory = new(Encoding.UTF8.GetBytes(rawData));
        return new(memory);
    }

    public static implicit operator string(Whisper whisper) => whisper.Content;
}