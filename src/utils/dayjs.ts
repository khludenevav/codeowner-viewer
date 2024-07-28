// eslint-disable-next-line no-restricted-imports
import originalDayjs from 'dayjs';
import advancedFormat from 'dayjs/plugin/advancedFormat';
import customParseFormat from 'dayjs/plugin/customParseFormat';
import duration from 'dayjs/plugin/duration';
import isSameOrAfter from 'dayjs/plugin/isSameOrAfter';
import isSameOrBefore from 'dayjs/plugin/isSameOrBefore';
import localizedFormat from 'dayjs/plugin/localizedFormat';
import minMax from 'dayjs/plugin/minMax';
import objectSupport from 'dayjs/plugin/objectSupport';
import relativeTime from 'dayjs/plugin/relativeTime';
import utc from 'dayjs/plugin/utc';

originalDayjs.extend(utc);
originalDayjs.extend(objectSupport);
originalDayjs.extend(isSameOrBefore);
originalDayjs.extend(isSameOrAfter);
originalDayjs.extend(duration);
originalDayjs.extend(customParseFormat);
originalDayjs.extend(relativeTime);
originalDayjs.extend(localizedFormat);
originalDayjs.extend(minMax);
originalDayjs.extend(advancedFormat);

declare module 'dayjs' {
  // Basic max and min declaration returns Dayjs or null. This override removes null from return type
  function max(dayjs: [Dayjs] | [Dayjs, Dayjs] | [Dayjs, Dayjs, Dayjs]): Dayjs;
  function min(dayjs: [Dayjs] | [Dayjs, Dayjs] | [Dayjs, Dayjs, Dayjs]): Dayjs;
}

export { originalDayjs as dayjs };
export type Duration = ReturnType<typeof originalDayjs.duration>;
